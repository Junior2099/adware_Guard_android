use crate::core::{self, Adb, Result};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BrowserTarget { pub user: String, pub package: String, pub default_confirmed: bool, pub explicit_choice: bool, pub cache_only_supported: bool }
pub const SUPPORTED_BROWSERS: [(&str,&str); 3] = [("Chrome","com.android.chrome"),("Brave","com.brave.browser"),("Firefox","org.mozilla.firefox")];
fn browser_allowed(package: &str) -> bool { SUPPORTED_BROWSERS.iter().any(|(_,p)| *p == package) }
const FILES_URL: &str = "https://play.google.com/store/apps/details?id=com.google.android.apps.nbu.files";
fn cache_clear_args(target: &BrowserTarget) -> Result<Vec<String>> {
    if !browser_allowed(&target.package) { return Err("Limpeza permitida somente para Chrome, Brave e Firefox oficiais (versões estáveis). Nenhum outro app será limpo.".into()); }
    if !target.cache_only_supported { return Err("Limpeza automática somente de cache indisponível.".into()); }
    if target.user.is_empty() || !target.user.bytes().all(|b| b.is_ascii_digit()) { return Err("Usuário Android inválido".into()); }
    Ok(["shell", "pm", "clear", "--user", &target.user, "--cache-only", &target.package].into_iter().map(str::to_string).collect())
}
fn browser_component(text: &str) -> Option<String> {
    let lines: Vec<_> = text.lines().map(str::trim).filter(|l| !l.is_empty()).collect();
    if lines.len() != 1 { return None; }
    let (package, activity) = lines[0].split_once('/')?;
    if !core::valid_package(package) || package == "android" || activity.to_lowercase().contains("resolver") || activity.to_lowercase().contains("chooser") { return None; }
    Some(package.into())
}
fn role_browser(text: &str) -> Option<String> {
    let lines: Vec<_> = text.lines().map(str::trim).filter(|l| !l.is_empty()).collect();
    if lines.len() == 1 && core::valid_package(lines[0]) && lines[0] != "android" { Some(lines[0].into()) } else { None }
}
fn cache_support(sdk: &str, help: &str) -> bool {
    sdk.trim().parse::<u32>().is_ok_and(|v| v >= 33) && help.lines().any(|l| {
        let l = l.trim(); l.starts_with("clear ") && l.contains("[--cache-only]") && l.contains("PACKAGE")
    })
}
impl Adb {
    pub fn browser_target(&self) -> Result<BrowserTarget> {
        let user = self.user()?;
        let role = self.run(&["shell", "cmd", "role", "get-role-holders", "--user", &user, "android.app.role.BROWSER"]).ok().and_then(|s| role_browser(&s));
        let detected = role.or_else(|| self.run(&["shell", "cmd", "package", "resolve-activity", "--brief", "--user", &user,
            "-a", "android.intent.action.VIEW", "-c", "android.intent.category.BROWSABLE", "-d", "https://example.com"]).ok().and_then(|s|browser_component(&s)));
        let (package, default_confirmed) = if let Some(package) = detected { (package,true) }
            else { ("com.android.chrome".into(),false) };
        self.browser_details(user, package, default_confirmed, false)
    }
    pub fn browser_named(&self, package: &str) -> Result<BrowserTarget> {
        if !browser_allowed(package) { return Err("Navegador fora da lista permitida de limpeza.".into()); }
        self.browser_details(self.user()?, package.into(), false, true)
    }
    fn browser_details(&self, user: String, package: String, default_confirmed: bool, explicit_choice: bool) -> Result<BrowserTarget> {
        if !browser_allowed(&package) { return Err(format!("O padrão detectado ({package}) não está na lista de limpeza. Use os botões Chrome, Brave ou Firefox para escolher um deles.")); }
        if !self.packages(&user, false)?.contains(&package) && !self.packages(&user, true)?.contains(&package) {
            return Err(format!("{package} não está instalado para o usuário {user}. Nenhum cache foi apagado."));
        }
        let sdk = self.run(&["shell", "getprop", "ro.build.version.sdk"]).unwrap_or_default();
        let help = self.run(&["shell", "pm", "help"]).unwrap_or_default();
        Ok(BrowserTarget { user, package, default_confirmed, explicit_choice, cache_only_supported: cache_support(&sdk,&help) })
    }
    pub fn clear_browser_cache(&self, target: &BrowserTarget) -> Result<String> {
        // Re-detect everything immediately before a mutating command. Never run a bare pm clear.
        let current = if target.explicit_choice { self.browser_named(&target.package)? } else { self.browser_target()? };
        if &current != target { return Err("O navegador, usuário ou suporte mudou. Detecte o navegador novamente.".into()); }
        if !current.cache_only_supported { return Err("Limpeza automática somente de cache indisponível. Use a tela de armazenamento do navegador no celular.".into()); }
        let args = cache_clear_args(target)?;
        let output = self.run(&args.iter().map(String::as_str).collect::<Vec<_>>())?;
        if output.trim() != "Success" { return Err(format!("Limpeza de cache não confirmada pelo Android: {output}")); }
        Ok(format!("Android informou sucesso na limpeza somente do cache de {} (usuário {}). Cookies, histórico e armazenamento não foram solicitados para exclusão. O navegador pode recriar cache durante o uso; não foi medida a quantidade liberada.",target.package,target.user))
    }
    pub fn open_browser_storage(&self, target: &BrowserTarget) -> Result<String> {
        if self.user()? != target.user || !browser_allowed(&target.package) { return Err("Usuário mudou ou pacote inválido. Detecte novamente.".into()); }
        let uri = format!("package:{}",target.package);
        let result = self.run(&["shell", "am", "start", "--user", &target.user, "-a", "android.settings.APPLICATION_DETAILS_SETTINGS", "-d", &uri])?;
        if result.contains("Error") || result.contains("Exception") { return Err(result); }
        Ok(format!("Tela de {} solicitada no celular. Abra Armazenamento → Limpar cache. Não escolha Limpar dados/armazenamento. Nenhum cache foi apagado automaticamente.",target.package))
    }
    pub fn install_files_page(&self) -> Result<String> {
        let user = self.user()?;
        let play = self.run(&["shell", "am", "start", "--user", &user, "-a", "android.intent.action.VIEW", "-d", FILES_URL, "-p", "com.android.vending"]);
        if play.as_ref().is_ok_and(|text| !text.contains("Error") && !text.contains("Exception")) {
            return Ok("Página oficial do Files do Google solicitada na Play Store do celular. Toque em Instalar (ou Abrir/Atualizar se já instalado). O AdwareGuard não instalou nem concedeu permissões automaticamente e não apagou arquivos.".into());
        }
        let web = self.run(&["shell", "am", "start", "--user", &user, "-a", "android.intent.action.VIEW", "-d", FILES_URL])?;
        if web.contains("Error") || web.contains("Exception") { return Err(format!("Não foi possível abrir a página oficial do Files. Abra no celular: {FILES_URL}. Nenhum arquivo foi apagado.")); }
        Ok("Play Store indisponível ou recusou abrir. A página oficial do Files foi solicitada no navegador do celular; conclua a instalação pela loja, se disponível. Nenhum APK foi baixado pelo AdwareGuard, nenhum arquivo foi apagado.".into())
    }
    pub fn open_accessibility(&self) -> Result<String> {
        let user = self.user()?;
        let result = self.run(&["shell", "am", "start", "--user", &user, "-a", "android.settings.ACCESSIBILITY_SETTINGS"])?;
        if result.contains("Error") || result.contains("Exception") { return Err(result); }
        Ok("Acessibilidade solicitada no celular. Confira Reduzir/Remover animações, possivelmente em Melhorias de visibilidade ou Cor e movimento.".into())
    }
    pub fn accessibility_motion(&self, disabled: bool) -> Result<String> {
        let user = self.user()?;
        // OEM keys are optional: never create keys on devices that don't expose them.
        let mut exposed = Vec::new();
        for (namespace,key) in [("global","remove_animations"),("system","reduce_animations")] {
            let old = self.run(&["shell", "settings", "--user", &user, "get", namespace, key])?;
            if old == "0" || old == "1" { exposed.push((namespace,key,old)); }
        }
        if self.user()? != user { return Err("Usuário Android mudou. Tente novamente.".into()); }
        let base = self.animations(disabled)?;
        let desired = if disabled { "1" } else { "0" };
        let mut written = Vec::new();
        for (namespace,key,old) in &exposed {
            // Add before writing because a timeout may mean the write was applied.
            written.push((*namespace,*key,old.as_str()));
            let result = self.run(&["shell", "settings", "--user", &user, "put", namespace, key, desired]).and_then(|_| {
                let actual = self.run(&["shell", "settings", "--user", &user, "get", namespace, key])?;
                if actual == desired { Ok(()) } else { Err(format!("{namespace}/{key} não confirmou o ajuste")) }
            });
            if let Err(error) = result {
                let mut restored = true;
                for (namespace,key,old) in written {
                    let check = self.run(&["shell", "settings", "--user", &user, "put", namespace, key, old])
                        .and_then(|_| self.run(&["shell", "settings", "--user", &user, "get", namespace, key]));
                    restored &= check.is_ok_and(|actual| actual == old);
                }
                return Err(format!("As três escalas já foram ajustadas, mas o ajuste extra de acessibilidade falhou: {error}. Restauração dos ajustes extras verificada: {restored}. Confira no celular."));
            }
        }
        Ok(format!("{base} Ajustes extras de acessibilidade lidos e verificados: {}. No Android padrão, Remover animações usa essas três escalas. Opções do fabricante são tratadas quando expostas; confirme o botão no menu Acessibilidade do celular.",exposed.len()))
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test] fn cache_commands_allow_only_requested_browsers_and_never_protected_apps_or_paths() {
        let mut t = BrowserTarget { user: "10".into(), package: String::new(), default_confirmed: false, explicit_choice: true, cache_only_supported: true };
        for (_,package) in SUPPORTED_BROWSERS {
            t.package = package.into(); let args = cache_clear_args(&t).unwrap();
            assert_eq!(args, ["shell","pm","clear","--user","10","--cache-only",package]);
        }
        for package in ["com.whatsapp","com.whatsapp.w4b","com.instagram.android","com.facebook.katana","com.facebook.lite","com.nu.production","com.itau","com.google.android.apps.nbu.files","/sdcard/Download","/sdcard/WhatsApp","com.unknown.bank"] {
            t.package = package.into(); assert!(cache_clear_args(&t).is_err(), "{package}");
        }
        t.package = "com.brave.browser".into(); t.cache_only_supported = false; assert!(cache_clear_args(&t).is_err());
        t.cache_only_supported = true; t.user = "10;reboot".into(); assert!(cache_clear_args(&t).is_err());
    }
    #[test] fn cache_cleanup_requires_explicit_supported_syntax_and_modern_android() {
        let help = " clear [--user USER_ID] [--cache-only] PACKAGE\n";
        assert!(cache_support("33",help)); assert!(!cache_support("32",help));
        assert!(!cache_support("35","clear [--user USER_ID] PACKAGE"));
        assert!(!cache_support("35","some command --cache-only")); assert!(!cache_support("unknown",help));
    }
    #[test] fn browsers_must_be_unambiguous_and_never_resolver_or_assumed_chrome() {
        assert_eq!(browser_component("com.brave.browser/org.chromium.Main").as_deref(),Some("com.brave.browser"));
        assert!(browser_component("android/com.android.internal.app.ResolverActivity").is_none());
        assert!(browser_component("com.foo.app/.ChooserActivity").is_none());
        assert!(role_browser("com.chrome.app\ncom.other.app").is_none());
        assert!(role_browser("Error: no role").is_none());
        assert_eq!(role_browser("org.mozilla.firefox").as_deref(),Some("org.mozilla.firefox"));
    }
}
