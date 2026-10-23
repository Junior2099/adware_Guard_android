//! Strict allowlist for regenerable shared thumbnail cache. No recursive deletion.
use crate::core::{Adb, Result};
use std::collections::BTreeSet;
#[derive(Clone, Debug)]
pub struct CleanupPlan { pub user: String, pub paths: Vec<String>, pub notes: Vec<String> }
fn roots(user: &str) -> Result<Vec<String>> {
    if user.is_empty() || !user.bytes().all(|b|b.is_ascii_digit()) { return Err("Usuário Android inválido".into()); }
    Ok(["DCIM","Pictures"].into_iter().map(|d|format!("/storage/emulated/{user}/{d}/.thumbnails")).collect())
}
fn allowed(user: &str, path: &str) -> bool {
    let Ok(roots) = roots(user) else { return false; };
    for root in roots {
        let Some(name) = path.strip_prefix(&(root+"/")) else { continue; };
        if name.is_empty() || !name.bytes().all(|b|b.is_ascii_alphanumeric() || b"._-".contains(&b)) { return false; }
        let lower = name.to_ascii_lowercase();
        return lower.starts_with(".thumbdata") || [".jpg",".jpeg",".png",".webp"].iter().any(|ext|lower.ends_with(ext));
    }
    false
}
fn removal_script(user: &str, paths: &[String]) -> Result<String> {
    // Validate the entire batch before generating any command. No arbitrary file paths.
    if paths.is_empty() || paths.len() > 50 || paths.iter().any(|p| !allowed(user,p)) { return Err("Lote de limpeza fora das pastas de miniaturas permitidas.".into()); }
    let items = paths.iter().map(|p|format!("'{p}'")).collect::<Vec<_>>().join(" ");
    Ok(format!(r#"for p in {items}; do parent=${{p%/*}}; base=${{parent%/*}}; if [ ! -L "$base" ] && [ ! -L "$parent" ] && [ -f "$p" ] && [ ! -L "$p" ]; then if rm -- "$p" && [ ! -e "$p" ]; then printf '%s\n' "AG_REMOVED:$p"; else printf '%s\n' "AG_FAILED:$p"; fi; else printf '%s\n' "AG_FAILED:$p"; fi; done"#))
}
impl Adb {
    pub fn device_cache_plan(&self) -> Result<CleanupPlan> {
        let user = self.user()?; let mut paths = BTreeSet::new(); let mut notes = Vec::new();
        for root in roots(&user)? {
            let parent = root.rsplit_once('/').unwrap().0;
            let script = format!("if [ -d '{root}' ] && [ ! -L '{root}' ] && [ ! -L '{parent}' ]; then find '{root}' -maxdepth 1 -type f; else echo AG_UNAVAILABLE; fi");
            match self.run(&["shell", &script]) {
                Ok(output) => {
                    if output.trim() == "AG_UNAVAILABLE" { notes.push(format!("{root}: ausente, sem acesso ou não elegível.")); }
                    else {
                        for path in output.lines().map(str::trim) {
                            if allowed(&user,path) { paths.insert(path.to_string()); }
                            else if !path.is_empty() { notes.push(format!("Entrada ignorada em {root}: formato fora da lista permitida.")); }
                        }
                    }
                }
                Err(error) => notes.push(format!("Não foi possível ler {root}: {error}")),
            }
        }
        let total = paths.len();
        if total > 1000 { notes.push(format!("Encontrados {total} candidatos; esta prévia limita a remoção aos primeiros 1000. Reanalise para continuar.")); }
        Ok(CleanupPlan { user, paths: paths.into_iter().take(1000).collect(), notes })
    }
    pub fn clean_device_cache(&self, plan: &CleanupPlan) -> Result<String> {
        if self.user()? != plan.user { return Err("Usuário Android mudou. Gere outra prévia.".into()); }
        if plan.paths.iter().any(|p|!allowed(&plan.user,p)) { return Err("Prévia contém caminho não permitido. Nenhum arquivo foi removido.".into()); }
        let paths: Vec<_> = plan.paths.iter().cloned().collect::<BTreeSet<_>>().into_iter().collect();
        if paths.is_empty() { return Ok("Nenhuma miniatura elegível encontrada; nenhum arquivo foi removido. Isso não significa que o aparelho esteja sem cache.".into()); }
        let mut removed = BTreeSet::new(); let mut errors = Vec::new();
        for chunk in paths.chunks(50) {
            match self.user() {
                Ok(user) if user == plan.user => {},
                Ok(_) => { errors.push("Usuário mudou durante a limpeza; o restante foi interrompido.".into()); break; },
                Err(error) => { errors.push(format!("Conexão interrompida; parte do lote pode ter sido aplicada: {error}")); break; },
            }
            let script = removal_script(&plan.user,chunk)?;
            match self.run(&["shell", &script]) {
                Ok(output) => for line in output.lines() {
                    if let Some(path) = line.strip_prefix("AG_REMOVED:") { if chunk.iter().any(|p|p == path) { removed.insert(path.to_string()); } }
                },
                Err(error) => errors.push(error),
            }
        }
        Ok(format!("Limpeza de miniaturas: {}/{} arquivos com remoção verificada; {} não removidos ou sem confirmação. Não foi medido espaço liberado. Downloads, WhatsApp, caches de bancos, Instagram e Facebook não foram alvos. As miniaturas podem ser recriadas. {}",removed.len(),paths.len(),paths.len()-removed.len(),errors.join(" | ")))
    }
    pub fn open_play_protect(&self) -> Result<String> {
        let user = self.user()?;
        let output = self.run(&["shell", "am", "start", "--user", &user, "-a", "android.intent.action.MAIN", "-c", "android.intent.category.LAUNCHER", "-p", "com.android.vending"])?;
        if output.contains("Error") || output.contains("Exception") { return Err("A Play Store não pôde ser aberta. Play Protect depende dos serviços Google. Nenhuma verificação foi iniciada pelo AdwareGuard.".into()); }
        Ok("Play Store solicitada no celular. Toque no perfil → Play Protect → Verificar. Aguarde o resultado e siga as recomendações do Google. O AdwareGuard não iniciou uma varredura automaticamente, não lê esse resultado e não contou apps removidos pelo Play Protect.".into())
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test] fn cleanup_is_limited_to_direct_thumbnail_files_for_the_current_user() {
        for p in ["/storage/emulated/10/DCIM/.thumbnails/.thumbdata4--12", "/storage/emulated/10/Pictures/.thumbnails/image.JPG"] { assert!(allowed("10",p)); }
        for p in ["/storage/emulated/0/DCIM/.thumbnails/x.jpg", "/storage/emulated/10/DCIM/photo.jpg", "/storage/emulated/10/Download/x.jpg", "/storage/emulated/10/Downloads/x.jpg", "/storage/emulated/10/WhatsApp/x.jpg", "/storage/emulated/10/Android/data/com.whatsapp/cache/x.jpg", "/storage/emulated/10/Android/data/com.instagram.android/cache/x.jpg", "/storage/emulated/10/Android/data/com.facebook.katana/cache/x.jpg", "/storage/emulated/10/Android/data/com.bank/cache/x.jpg", "/storage/emulated/10/DCIM/.thumbnails/../photo.jpg", "/storage/emulated/10/DCIM/.thumbnails/subdir/x.jpg", "/storage/emulated/10/DCIM/.thumbnails/a;reboot.jpg", "/storage/emulated/10/DCIM/.thumbnails/important.pdf"] { assert!(!allowed("10",p),"{p}"); }
        assert!(roots("10;reboot").is_err());
    }
    #[cfg(unix)]
    #[test] fn shell_cleanup_removes_only_regular_thumbnail_files_and_refuses_symlinks() {
        use std::{fs, os::unix::fs::symlink, process::Command};
        let dir = tempfile::tempdir().unwrap();
        let cache = dir.path().join("DCIM/.thumbnails"); fs::create_dir_all(&cache).unwrap();
        fs::create_dir(dir.path().join("Download")).unwrap();
        fs::write(cache.join("a.jpg"),b"thumbnail").unwrap();
        fs::write(dir.path().join("DCIM/original.jpg"),b"original").unwrap();
        fs::write(dir.path().join("Download/b.jpg"),b"download").unwrap();
        symlink(dir.path().join("Download/b.jpg"),cache.join("b.jpg")).unwrap();
        let paths = ["a.jpg","b.jpg"].map(|n|format!("/storage/emulated/0/DCIM/.thumbnails/{n}"));
        let script = removal_script("0",&paths).unwrap().replace("/storage/emulated/0",dir.path().to_str().unwrap());
        let output = Command::new("sh").args(["-c",&script]).output().unwrap(); assert!(output.status.success());
        assert!(!cache.join("a.jpg").exists()); assert!(cache.join("b.jpg").is_symlink());
        assert!(dir.path().join("DCIM/original.jpg").exists()); assert!(dir.path().join("Download/b.jpg").exists());
        fs::remove_file(cache.join("b.jpg")).unwrap(); fs::remove_dir(&cache).unwrap();
        symlink(dir.path().join("Download"),&cache).unwrap();
        let script = removal_script("0",&[paths[1].clone()]).unwrap().replace("/storage/emulated/0",dir.path().to_str().unwrap());
        assert!(Command::new("sh").args(["-c",&script]).status().unwrap().success());
        assert!(dir.path().join("Download/b.jpg").exists());
    }
    #[test] fn cleanup_rejects_an_entire_batch_if_any_path_is_outside_allowlist() {
        let good = "/storage/emulated/0/DCIM/.thumbnails/a.jpg".to_string();
        let bad = "/storage/emulated/0/Download/b.jpg".to_string();
        assert!(removal_script("0",&[good.clone(),bad]).is_err());
        let script = removal_script("0",&[good]).unwrap();
        assert!(script.contains("rm --")); assert!(!script.contains("rm -r")); assert!(script.contains("! -L"));
    }
}
