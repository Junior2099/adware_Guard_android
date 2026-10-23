//! Optional metadata retrieval. APKs are read locally and deleted with the temp directory.
use crate::core::{self, Adb};
use std::{io::{Cursor, Read}, path::PathBuf};
pub struct Metadata { pub label: String, pub pixels: Option<(usize, usize, Vec<u8>)>, pub note: String }
fn quoted(s: &str) -> Option<&str> { s.split_once('\'')?.1.split_once('\'').map(|(v,_)| v) }
pub fn load(adb: &Adb, aapt: PathBuf, user: String, package: String) -> core::Result<Metadata> {
    if !core::valid_package(&package) { return Err("Pacote inválido".into()); }
    if !aapt.is_file() { return Err("Para ícones reais, selecione aapt2.exe em Conexão → Caminhos avançados. A análise continua funcionando sem ele.".into()); }
    if adb.user()? != user { return Err("Usuário Android mudou. Analise novamente.".into()); }
    let paths = adb.run(&["shell", "pm", "path", "--user", &user, &package])?;
    let paths: Vec<_> = paths.lines().filter_map(|l| l.trim().strip_prefix("package:")).collect();
    let remote = paths.iter().find(|p| p.ends_with("/base.apk")).or_else(|| paths.first()).ok_or("Caminho do APK não disponível")?;
    let dir = tempfile::tempdir().map_err(|e| e.to_string())?;
    let apk = dir.path().join("base.apk");
    let local = apk.to_str().ok_or("Caminho temporário inválido")?;
    adb.run(&["pull", remote, local])?;
    let text = core::command(&aapt, &["dump", "badging", local])?;
    let label = text.lines().find_map(|l| l.strip_prefix("application-label-pt-BR:").and_then(quoted))
        .or_else(|| text.lines().find_map(|l| l.strip_prefix("application-label:").and_then(quoted)))
        .unwrap_or(&package).to_owned();
    let mut resources: Vec<String> = text.lines().filter(|l| l.starts_with("application-icon-"))
        .filter_map(quoted).filter(|s| s.ends_with(".png") || s.ends_with(".webp") || s.ends_with(".jpg")).map(str::to_owned).collect();
    if let Some(path) = text.lines().find(|l| l.starts_with("application:")).and_then(|l| l.split_once("icon='")).and_then(|(_,r)|r.split_once('\'').map(|(v,_)|v)) {
        if !resources.iter().any(|r| r == path) { resources.push(path.to_owned()); }
    }
    let file = std::fs::File::open(&apk).map_err(|e| e.to_string())?;
    let mut archive = zip::ZipArchive::new(file).map_err(|e| e.to_string())?;
    for path in resources.iter().rev() {
        let Ok(mut entry) = archive.by_name(path) else { continue; };
        if entry.size() > 2_000_000 { continue; }
        let mut bytes = Vec::new();
        if (&mut entry).take(2_000_001).read_to_end(&mut bytes).is_err() || bytes.len() > 2_000_000 { continue; }
        let Ok(reader) = image::ImageReader::new(Cursor::new(&bytes)).with_guessed_format() else { continue; };
        let Ok((w,h)) = reader.into_dimensions() else { continue; };
        if w == 0 || h == 0 || w > 1024 || h > 1024 { continue; }
        let Ok(decoded) = image::load_from_memory(&bytes) else { continue; };
        let rgba = decoded.thumbnail(96, 96).to_rgba8();
        return Ok(Metadata { label, pixels: Some((rgba.width() as usize, rgba.height() as usize, rgba.into_raw())), note: "Ícone obtido do APK instalado.".into() });
    }
    Ok(Metadata { label, pixels: None, note: "Nome carregado. Ícone adaptativo/XML ou recurso em split APK indisponível; exibindo identificação alternativa.".into() })
}
