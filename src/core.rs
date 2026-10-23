use std::{collections::BTreeSet, path::PathBuf, process::{Command, Stdio}, thread, time::{Duration, Instant}};

pub type Result<T> = std::result::Result<T, String>;
#[derive(Clone, Debug)]
pub struct Device { pub serial: String, pub state: String }
#[derive(Clone, Debug)]
pub struct Package { pub name: String, pub system: bool, pub reason: String, pub matched: bool, pub removed: bool }
#[derive(Clone)]
pub struct Adb { pub path: PathBuf, pub serial: String }

pub fn root() -> PathBuf {
    let exe = std::env::current_exe().ok().and_then(|p| p.parent().map(|p| p.to_path_buf()));
    if let Some(p) = exe { if p.join("tools/platform-tools").is_dir() { return p; } }
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}
pub fn valid_package(s: &str) -> bool {
    // O framework Android usa o identificador especial sem ponto: android.
    s == "android" || s.contains('.') && s.split('.').all(|part| !part.is_empty() && part.bytes().all(|c| c.is_ascii_alphanumeric() || c == b'_'))
}
pub(crate) fn command(path: &PathBuf, args: &[&str]) -> Result<String> {
    let mut cmd = Command::new(path);
    cmd.args(args).stdin(Stdio::null()).stdout(Stdio::piped()).stderr(Stdio::piped());
    #[cfg(windows)] {
        use std::os::windows::process::CommandExt;
        cmd.creation_flags(0x08000000);
    }
    let mut child = cmd.spawn().map_err(|e| format!("Não foi possível iniciar ADB em {}: {e}", path.display()))?;
    // Drain both pipes concurrently to avoid deadlock on large package lists.
    let out = child.stdout.take().ok_or("Saída ADB indisponível")?;
    let err = child.stderr.take().ok_or("Erros ADB indisponíveis")?;
    fn read(mut p: impl std::io::Read) -> std::io::Result<Vec<u8>> { let mut b = Vec::new(); p.read_to_end(&mut b)?; Ok(b) }
    let stdout = thread::spawn(move || read(out));
    let stderr = thread::spawn(move || read(err));
    let started = Instant::now();
    let status = loop {
        match child.try_wait() {
            Ok(Some(s)) => break s,
            Ok(None) if started.elapsed() < Duration::from_secs(45) => thread::sleep(Duration::from_millis(60)),
            _ => { let _ = child.kill(); let _ = child.wait(); return Err("ADB excedeu 45 segundos ou falhou. A operação pode ter sido aplicada; analise novamente antes de repetir.".into()); }
        }
    };
    let output = stdout.join().map_err(|_| "Falha lendo ADB")?.map_err(|e| e.to_string())?;
    let errors = stderr.join().map_err(|_| "Falha lendo ADB")?.map_err(|e| e.to_string())?;
    if !status.success() { return Err(format!("ADB: {} {}", String::from_utf8_lossy(&output), String::from_utf8_lossy(&errors))); }
    Ok(String::from_utf8_lossy(&output).trim().to_string())
}
impl Adb {
    pub fn run(&self, args: &[&str]) -> Result<String> {
        let mut all = vec!["-s", self.serial.as_str()]; all.extend_from_slice(args); command(&self.path, &all)
    }
    pub fn devices(&self) -> Result<Vec<Device>> {
        Ok(command(&self.path, &["devices"] )?.lines().filter_map(|l| {
            let mut w = l.split_whitespace(); let serial = w.next()?; let state = w.next()?;
            if serial == "List" || l.starts_with('*') { return None; }
            Some(Device { serial: serial.into(), state: state.into() })
        }).collect())
    }
    pub fn user(&self) -> Result<String> {
        let s = self.run(&["shell", "am", "get-current-user"])?;
        if !s.is_empty() && s.bytes().all(|b| b.is_ascii_digit()) { Ok(s) } else { Err(format!("Usuário Android inválido: {s}")) }
    }
    pub fn packages(&self, user: &str, system: bool) -> Result<BTreeSet<String>> {
        let flag = if system { "-s" } else { "-3" };
        parse_packages(&self.run(&["shell", "pm", "list", "packages", flag, "--user", user])?)
    }
    pub fn scan(&self) -> Result<(String, Vec<Package>)> {
        let user = self.user()?;
        let third = self.packages(&user, false)?;
        let system = self.packages(&user, true)?;
        let rules_path = root().join("data/confirmed_malware.txt");
        let rules = std::fs::read_to_string(&rules_path).map_err(|e| format!("Lista indisponível ({}): {e}", rules_path.display()))?;
        let rules = parse_rules(&rules)?;
        let mut all = Vec::new();
        for name in third.union(&system) {
            let matched = rules.contains(name);
            let reason = if matched { "Correspondência exata na lista local; confirme a evidência e a origem.".into() }
                else if ["adware", "popup", "pushads", "clicker"].iter().any(|s| name.to_lowercase().contains(s)) {
                    "Nome contém termo de publicidade; sinal fraco, sujeito a falso positivo.".into()
                } else { "Sem indício pelas regras disponíveis; não garante segurança.".into() };
            all.push(Package { name: name.clone(), system: name == "android" || system.contains(name), reason, matched, removed: false });
        }
        all.sort_by_key(|p| (!p.matched, !p.reason.starts_with("Nome"), p.name.clone()));
        Ok((user, all))
    }
    pub fn remove(&self, user: &str, package: &str) -> Result<()> {
        if !valid_package(package) { return Err("Nome de pacote inválido".into()); }
        if package == "android" { return Err("O framework Android é protegido contra remoção.".into()); }
        if self.user()? != user { return Err("O usuário Android mudou. Analise novamente.".into()); }
        if self.packages(user, true)?.contains(package) { return Err("Remoção de aplicativo do sistema bloqueada.".into()); }
        if !self.packages(user, false)?.contains(package) { return Err("Aplicativo não está mais instalado. Analise novamente.".into()); }
        let output = self.run(&["shell", "pm", "uninstall", "--user", user, package])?;
        if !output.lines().any(|l| l.trim() == "Success") { return Err(format!("Remoção não confirmada: {output}")); }
        if self.packages(user, false)?.contains(package) { return Err("ADB retornou sucesso, mas o pacote ainda aparece instalado.".into()); }
        Ok(())
    }
    pub fn dns(&self, automatic: bool) -> Result<String> {
        let old_mode = self.run(&["shell", "settings", "get", "global", "private_dns_mode"])?;
        let old_host = self.run(&["shell", "settings", "get", "global", "private_dns_specifier"])?;
        let change = (|| {
            if !automatic { self.run(&["shell", "settings", "put", "global", "private_dns_specifier", "dns.adguard-dns.com"])?; }
            let mode = if automatic { "opportunistic" } else { "hostname" };
            self.run(&["shell", "settings", "put", "global", "private_dns_mode", mode])?;
            let actual = self.run(&["shell", "settings", "get", "global", "private_dns_mode"])?;
            let host = self.run(&["shell", "settings", "get", "global", "private_dns_specifier"])?;
            if actual != mode || (!automatic && host != "dns.adguard-dns.com") { return Err("Leitura não confirmou os valores".into()); }
            Ok("Valores do DNS gravados e lidos. Confira a conexão e o menu DNS privado no celular; a conectividade não foi testada.".into())
        })();
        if let Err(e) = change {
            // Restore known prior values; never forward arbitrary settings output to a remote shell.
            let mut restored = true;
            for (key, value) in [("private_dns_specifier", old_host.as_str()), ("private_dns_mode", old_mode.as_str())] {
                let safe = value.bytes().all(|b| b.is_ascii_alphanumeric() || b".-_".contains(&b));
                let r = if value == "null" || value.is_empty() { self.run(&["shell", "settings", "delete", "global", key]) }
                    else if safe { self.run(&["shell", "settings", "put", "global", key, value]) }
                    else { Err("Valor anterior inesperado".into()) };
                restored &= r.is_ok();
            }
            return Err(format!("DNS: {e}. Restauração solicitada: {restored}. Confira manualmente no Android."));
        }
        change
    }
}
pub fn parse_packages(s: &str) -> Result<BTreeSet<String>> {
    let mut set = BTreeSet::new();
    for line in s.lines().map(str::trim).filter(|l| !l.is_empty()) {
        let name = line.strip_prefix("package:").ok_or_else(|| format!("Resposta inesperada: {line}"))?;
        if !valid_package(name) { return Err(format!("Pacote inválido: {name}")); }
        set.insert(name.into());
    }
    Ok(set)
}
pub fn parse_rules(s: &str) -> Result<BTreeSet<String>> {
    let mut set = BTreeSet::new();
    for (i, line) in s.lines().enumerate() {
        let line = line.trim(); if line.is_empty() || line.starts_with('#') { continue; }
        let (package, source) = line.split_once('|').ok_or_else(|| format!("Lista: linha {} sem fonte", i+1))?;
        let package = package.trim();
        if !valid_package(package) || !source.trim().starts_with("https://") { return Err(format!("Lista: linha {} inválida", i+1)); }
        set.insert(package.into());
    }
    Ok(set)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test] fn packages_reject_shell_and_errors() {
        assert!(!valid_package("com.app;reboot")); assert!(!valid_package("com..app"));
        assert!(parse_packages("Error: denied").is_err());
        assert_eq!(parse_packages("package:com.test.app\npackage:com.test.app").unwrap().len(), 1);
    }
    #[test] fn framework_package_is_valid_and_preserved() {
        assert!(valid_package("android"));
        let packages = parse_packages("package:android\npackage:com.android.settings\r\n").unwrap();
        assert!(packages.contains("android"));
        assert_eq!(packages.len(), 2);
        assert!(!valid_package("other"));
        assert!(!valid_package("android;reboot"));
        assert!(parse_packages("package:android\nError: denied").is_err());
    }
    #[test] fn list_needs_evidence_and_exact_match() {
        assert!(parse_rules("com.test.app").is_err());
        let rules = parse_rules("# empty\ncom.test.app | https://example.org/report").unwrap();
        assert!(rules.contains("com.test.app")); assert!(!rules.contains("com.test.app2"));
        assert!(parse_rules(include_str!("../data/confirmed_malware.txt")).unwrap().is_empty());
    }
}
