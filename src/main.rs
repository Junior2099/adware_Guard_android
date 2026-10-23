#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
mod core;
mod icons;
mod maintenance;
mod device_cleanup;
use core::{Adb, Device, Package};
use eframe::egui::{self, Color32, RichText, Vec2};
use std::{collections::{HashMap, BTreeSet}, path::PathBuf, sync::mpsc::{self, Receiver}, time::Duration};
#[derive(Clone, Copy)]
struct Palette { bg: Color32, card: Color32, sidebar: Color32, text: Color32, muted: Color32, accent: Color32, danger: Color32, warning: Color32, button: Color32, avatar: Color32 }
fn palette(dark: bool) -> Palette {
    if dark { Palette { bg: Color32::from_rgb(13,20,33), card: Color32::from_rgb(22,32,48), sidebar: Color32::from_rgb(17,26,41), text: Color32::from_rgb(229,236,246), muted: Color32::from_rgb(154,172,194), accent: Color32::from_rgb(57,213,177), danger: Color32::from_rgb(243,123,131), warning: Color32::from_rgb(249,196,107), button: Color32::from_rgb(40,55,75), avatar: Color32::from_rgb(37,64,79) } }
    else { Palette { bg: Color32::from_rgb(240,244,249), card: Color32::WHITE, sidebar: Color32::from_rgb(230,237,246), text: Color32::from_rgb(24,40,61), muted: Color32::from_rgb(76,96,120), accent: Color32::from_rgb(0,112,89), danger: Color32::from_rgb(177,39,57), warning: Color32::from_rgb(132,82,0), button: Color32::from_rgb(221,230,240), avatar: Color32::from_rgb(213,239,232) } }
}
fn apply_theme(ctx: &egui::Context, dark: bool) {
    let c = palette(dark);
    let mut style = (*ctx.style()).clone();
    style.visuals = if dark { egui::Visuals::dark() } else { egui::Visuals::light() };
    style.visuals.panel_fill = c.bg; style.visuals.window_fill = c.card;
    style.visuals.override_text_color = Some(c.text);
    style.visuals.selection.bg_fill = if dark { Color32::from_rgb(37,105,98) } else { Color32::from_rgb(187,226,215) };
    style.spacing.item_spacing = Vec2::new(10.0,8.0);
    style.spacing.button_padding = Vec2::new(12.0,8.0);
    style.text_styles.insert(egui::TextStyle::Body, egui::FontId::proportional(14.0));
    ctx.set_style(style);
}
fn theme_toggle(ui: &mut egui::Ui, dark: &mut bool) -> bool {
    let colors = palette(*dark);
    let mut changed = false;
    ui.horizontal(|ui| {
        let (rect, mut response) = ui.allocate_exact_size(Vec2::new(44.0,24.0), egui::Sense::click());
        response.widget_info(|| egui::WidgetInfo::selected(egui::WidgetType::Checkbox, ui.is_enabled(), *dark, "Tema escuro"));
        if response.clicked() { *dark = !*dark; response.mark_changed(); changed = true; }
        let progress = ui.ctx().animate_bool(response.id, *dark);
        ui.painter().rect_filled(rect, 12, if *dark { colors.accent } else { Color32::from_rgb(135,153,174) });
        ui.painter().circle_filled(egui::pos2(rect.left()+12.0+progress*20.0, rect.center().y), 9.0, Color32::WHITE);
        if response.has_focus() { ui.painter().rect_stroke(rect.expand(2.0), 14, egui::Stroke::new(1.0_f32, colors.text), egui::StrokeKind::Outside); }
        response.on_hover_text("Alternar entre tema claro e escuro");
        ui.label(if *dark { "Tema escuro" } else { "Tema claro" });
    });
    changed
}
const RISK_LABELS: [&str; 5] = ["Todos", "Lista local", "Suspeitas por nome", "Sem indícios", "Removidos"];
fn risk(p: &Package) -> usize {
    if p.removed { 4 } else if p.matched { 1 } else if p.reason.starts_with("Nome") { 2 } else { 3 }
}
fn risk_description(category: usize) -> &'static str {
    match category {
        1 => "Correspondência na lista local. Revise a fonte e a identidade do APK; não é confirmação de infecção.",
        2 => "Termo para revisão no pacote ou nome exibido carregado. Sinal fraco, sujeito a falsos positivos.",
        3 => "Nenhum indício pelas regras disponíveis. Isso não significa que o app seja seguro.",
        4 => "Remoções verificadas nesta análise, para o usuário Android selecionado.",
        _ => "Categorias de análise separadas do tipo de app. Pacotes do sistema continuam protegidos.",
    }
}
enum Reply { Devices(Vec<Device>), Scan(String, Vec<Package>), Batch(Vec<(Package, core::Result<()>)>), Message(String), Browser(maintenance::BrowserTarget), Cleanup(device_cleanup::CleanupPlan), Metadata(String, icons::Metadata) }
struct AppMetadata { label: String, texture: Option<egui::TextureHandle>, note: String }
struct App {
    dark: bool, app_kind: usize, path: String, aapt: String, devices: Vec<Device>, selected: String, user: String,
    packages: Vec<Package>, filter: String, category: usize, status: String, log: Vec<String>,
    receiver: Option<Receiver<core::Result<Reply>>>, pending: Option<Vec<Package>>, selected_apps: BTreeSet<String>, confirmed: bool,
    dns_confirm: Option<bool>, animations_confirm: Option<bool>, browser_confirm: Option<maintenance::BrowserTarget>, cleanup_confirm: Option<device_cleanup::CleanupPlan>, cleanup_checked: bool, scanned: bool, removed: usize, matched_removed: usize,
    metadata: HashMap<String, AppMetadata>,
}
impl Default for App {
    fn default() -> Self { Self {
        dark: false, app_kind: 0, path: core::root().join("tools/platform-tools/adb.exe").display().to_string(),
        aapt: core::root().join("tools/aapt2/aapt2.exe").display().to_string(),
        devices: vec![], selected: String::new(), user: String::new(), packages: vec![], filter: String::new(), category: 0,
        status: "Conecte o celular por USB para começar.".into(), log: vec![], receiver: None,
        pending: None, selected_apps: BTreeSet::new(), confirmed: false, dns_confirm: None, animations_confirm: None, browser_confirm: None, cleanup_confirm: None, cleanup_checked: false, scanned: false, removed: 0, matched_removed: 0, metadata: HashMap::new(),
    } }
}
fn panel(ui: &egui::Ui) -> egui::Frame { let colors = palette(ui.visuals().dark_mode); egui::Frame::new().fill(colors.card).corner_radius(12).inner_margin(16) }
fn button(ui: &mut egui::Ui, label: &str, accent: bool) -> egui::Response {
    let colors = palette(ui.visuals().dark_mode);
    let b = egui::Button::new(RichText::new(label).strong().color(if accent { if ui.visuals().dark_mode { colors.bg } else { Color32::WHITE } } else { colors.text }))
        .fill(if accent { colors.accent } else { colors.button }).min_size(Vec2::new(0.0, 36.0)).corner_radius(8);
    ui.add(b)
}
fn avatar(ui: &mut egui::Ui, name: &str, texture: Option<&egui::TextureHandle>, size: f32) {
    let colors = palette(ui.visuals().dark_mode);
    if let Some(t) = texture { ui.add(egui::Image::new((t.id(), Vec2::splat(size)))); }
    else {
        let (rect, response) = ui.allocate_exact_size(Vec2::splat(size), egui::Sense::hover());
        ui.painter().rect_filled(rect, 10, colors.avatar);
        let initial = name.rsplit('.').next().unwrap_or(name).chars().next().unwrap_or('A').to_uppercase().to_string();
        ui.painter().text(rect.center(), egui::Align2::CENTER_CENTER, initial, egui::FontId::proportional(size * 0.45), colors.accent);
        response.on_hover_text("Identificação alternativa; não é o ícone original do aplicativo.");
    }
}
fn metric(ui: &mut egui::Ui, title: &str, value: usize, color: Color32) {
    let colors = palette(ui.visuals().dark_mode);
    panel(ui).show(ui, |ui| { ui.set_min_width((ui.available_width()-32.0).max(80.0));
        ui.label(RichText::new(title).size(12.0).color(colors.muted));
        ui.label(RichText::new(value.to_string()).size(30.0).strong().color(color));
    });
}
impl App {
    fn adb(&self) -> Adb { Adb { path: PathBuf::from(self.path.trim()), serial: self.selected.clone() } }
    fn start(&mut self, job: impl FnOnce() -> core::Result<Reply> + Send + 'static) {
        let (tx, rx) = mpsc::channel(); self.receiver = Some(rx); self.status = "Executando… aguarde.".into();
        std::thread::spawn(move || { let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(job))
            .unwrap_or_else(|_| Err("Falha interna no trabalho ADB".into())); let _ = tx.send(result); });
    }
    fn message(&mut self, s: String) { self.status = s.clone(); self.log.push(s); }
    fn summary(&self) -> String {
        if !self.scanned { return "Nenhuma análise concluída.".into(); }
        let matched = self.packages.iter().filter(|p| p.matched && !p.removed).count();
        let suspicious = self.packages.iter().filter(|p| risk(p) == 2).count();
        let finding = if matched == 0 && self.matched_removed == 0 {
            "Não foi encontrado nenhum app nocivo pelas regras disponíveis. Isso não garante ausência de malware.".into()
        } else { format!("Correspondências na lista local ainda instaladas: {matched}.") };
        format!("Apps da lista de nocivos eliminados: {} • Outros apps removidos: {} • Suspeitas por nome: {suspicious}\n{finding}",
            self.matched_removed, self.removed - self.matched_removed)
    }
    fn apply_batch(&mut self, results: Vec<(Package, core::Result<()>)>) {
        let total = results.len(); let mut success = 0;
        for (snapshot, result) in results {
            match result {
                Ok(()) => {
                    if let Some(p) = self.packages.iter_mut().find(|p| p.name == snapshot.name) { p.removed = true; }
                    self.selected_apps.remove(&snapshot.name);
                    self.removed += 1; if snapshot.matched { self.matched_removed += 1; } success += 1;
                    self.log.push(format!("{}: remoção verificada para usuário {}.", snapshot.name, self.user));
                }
                Err(error) => self.log.push(format!("{}: não removido ou não verificado — {error}", snapshot.name)),
            }
        }
        self.message(format!("Remoção concluída: {success}/{total} verificadas; {} falhas ou resultados não verificados. Confira o histórico. {}", total-success, self.summary()));
    }
    fn sidebar(&mut self, ui: &mut egui::Ui) {
        let colors = palette(self.dark);
        ui.add_space(10.0);
        ui.label(RichText::new("CONEXÃO USB").color(colors.accent).size(12.0).strong());
        ui.add_space(12.0);
        if button(ui, "1. Detectar dispositivos", true).clicked() { let adb = self.adb(); self.start(move || adb.devices().map(Reply::Devices)); }
        ui.add_space(8.0);
        for d in &self.devices {
            ui.label(RichText::new(format!("{} · {}", d.serial, d.state)).small().color(colors.muted));
            if d.state == "unauthorized" { ui.colored_label(colors.danger, "Aceite a autorização no celular."); }
            if d.state == "offline" { ui.colored_label(colors.danger, "Reconecte o cabo USB."); }
        }
        let before = self.selected.clone();
        egui::ComboBox::from_id_salt("device").width(226.0)
            .selected_text(if self.selected.is_empty() { "Selecione um aparelho" } else { &self.selected }).show_ui(ui, |ui| {
                for d in self.devices.iter().filter(|d| d.state == "device") { ui.selectable_value(&mut self.selected, d.serial.clone(), &d.serial); }
            });
        if before != self.selected { self.packages.clear(); self.selected_apps.clear(); self.metadata.clear(); self.scanned = false; self.removed = 0; self.matched_removed = 0; }
        ui.add_space(12.0);
        ui.add_enabled_ui(!self.selected.is_empty(), |ui| {
            if button(ui, "2. Analisar aplicativos", true).clicked() {
                let adb = self.adb(); self.scanned = false; self.packages.clear(); self.selected_apps.clear();
                self.start(move || adb.scan().map(|(u,p)| Reply::Scan(u,p)));
            }
        });
        ui.add_space(24.0); ui.separator(); ui.add_space(16.0);
        ui.label(RichText::new("DNS PRIVADO").color(colors.accent).size(12.0).strong());
        ui.add_space(8.0); ui.label("Filtragem de anúncios e rastreadores por DNS.");
        ui.label(RichText::new("dns.adguard-dns.com").small().color(colors.muted)); ui.add_space(12.0);
        ui.add_enabled_ui(!self.selected.is_empty(), |ui| {
            if button(ui, "Configurar AdGuard DNS", false).clicked() { self.dns_confirm = Some(false); }
            ui.add_space(6.0);
            if ui.button("Voltar ao DNS automático").clicked() { self.dns_confirm = Some(true); }
        });
        ui.add_space(8.0); ui.label(RichText::new("Android 9+. O DNS não remove apps nem bloqueia todos os anúncios.").small().color(colors.muted));
        ui.add_space(20.0);
        ui.separator(); ui.add_space(10.0);
        ui.label(RichText::new("ANIMAÇÕES DO ANDROID").color(colors.accent).size(12.0).strong());
        ui.label(RichText::new("Reduza os efeitos de movimento do celular.").small().color(colors.muted));
        ui.add_enabled_ui(!self.selected.is_empty(), |ui| {
            if button(ui, "Ativar redução de animações", false).clicked() { self.animations_confirm = Some(true); }
            if ui.button("Conferir em Acessibilidade").clicked() { let adb = self.adb(); self.start(move || adb.open_accessibility().map(Reply::Message)); }
            if ui.button("Animações padrão (1×)").clicked() { self.animations_confirm = Some(false); }
        });
        ui.add_space(18.0);
        ui.separator(); ui.add_space(10.0);
        ui.label(RichText::new("NAVEGADOR").color(colors.accent).size(12.0).strong());
        ui.add_enabled_ui(!self.selected.is_empty(), |ui| {
            if button(ui, "Cache do navegador padrão", false).clicked() { let adb = self.adb(); self.start(move || adb.browser_target().map(Reply::Browser)); }
        });
        ui.add_enabled_ui(!self.selected.is_empty(), |ui| {
            ui.horizontal_wrapped(|ui| {
                for (label,package) in maintenance::SUPPORTED_BROWSERS {
                    if ui.button(label).on_hover_text(format!("Revisar limpeza somente do cache: {package}")).clicked() {
                        let adb = self.adb(); self.start(move || adb.browser_named(package).map(Reply::Browser));
                    }
                }
            });
        });
        ui.label(RichText::new("Somente cache de Chrome, Brave ou Firefox estáveis, com confirmação. Outros aplicativos e pastas não são limpos.").small().color(colors.muted));
        ui.add_space(16.0);
        ui.label(RichText::new("FILES DO GOOGLE").color(colors.accent).size(12.0).strong());
        ui.add_enabled_ui(!self.selected.is_empty(), |ui| {
            if button(ui, "Instalar Files pela Play Store", false).clicked() { let adb = self.adb(); self.start(move || adb.install_files_page().map(Reply::Message)); }
        });
        ui.label(RichText::new("Abre a página oficial no celular. Você confirma a instalação. Nenhuma limpeza é iniciada pelo AdwareGuard.").small().color(colors.muted));
        ui.add_space(18.0);
        ui.separator(); ui.add_space(10.0);
        ui.label(RichText::new("LIMPEZA DO APARELHO").color(colors.accent).size(12.0).strong());
        ui.add_enabled_ui(!self.selected.is_empty(), |ui| {
            if button(ui, "Limpar cache do aparelho", false).clicked() { let adb = self.adb(); self.start(move || adb.device_cache_plan().map(Reply::Cleanup)); }
        });
        ui.label(RichText::new("Prévia de miniaturas regeneráveis em DCIM/Pictures. Não limpa cache geral dos apps nem pastas pessoais.").small().color(colors.muted));
        ui.add_space(14.0);
        ui.label(RichText::new("GOOGLE PLAY PROTECT").color(colors.accent).size(12.0).strong());
        ui.add_enabled_ui(!self.selected.is_empty(), |ui| {
            if button(ui, "Verificar com Play Protect", false).clicked() { let adb = self.adb(); self.start(move || adb.open_play_protect().map(Reply::Message)); }
        });
        ui.label(RichText::new("Abre a Play Store. No celular: perfil → Play Protect → Verificar. O resultado aparece no aparelho.").small().color(colors.muted));
        ui.add_space(18.0);
        egui::CollapsingHeader::new("Caminhos avançados").show(ui, |ui| {
            ui.label("ADB executável"); ui.add(egui::TextEdit::singleline(&mut self.path).desired_width(220.0));
            ui.label("AAPT2 · opcional para ícones"); ui.add(egui::TextEdit::singleline(&mut self.aapt).desired_width(220.0));
            ui.label(RichText::new("Escolha aapt2.exe do Android SDK Build-Tools. Veja o README.").small().color(colors.muted));
        });
    }
}
impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        let colors = palette(self.dark);
        if let Some(rx) = &self.receiver {
            match rx.try_recv() {
                Ok(result) => {
                    self.receiver = None;
                    match result {
                        Err(e) => self.message(format!("Erro: {e}")),
                        Ok(Reply::Devices(d)) => {
                            self.metadata.clear(); self.devices = d; self.packages.clear(); self.selected_apps.clear(); self.scanned = false;
                            self.selected = self.devices.iter().find(|d| d.state == "device").map(|d| d.serial.clone()).unwrap_or_default();
                            self.removed = 0; self.matched_removed = 0;
                            self.message("Detecção concluída. Selecione um aparelho autorizado e analise.".into());
                        }
                        Ok(Reply::Scan(user, mut packages)) => {
                            if self.user != user { self.metadata.clear(); }
                            for p in &mut packages {
                                if !p.matched { p.reason = core::name_reason(&p.name, self.metadata.get(&p.name).map(|m|m.label.as_str())); }
                            }
                            self.user = user; self.packages = packages; self.scanned = true;
                            self.message(format!("Análise concluída: {} pacotes. {}", self.packages.len(), self.summary())); }
                        Ok(Reply::Batch(results)) => self.apply_batch(results),
                        Ok(Reply::Metadata(name, info)) => {
                            let texture = info.pixels.map(|(w,h,pixels)| ctx.load_texture(
                                format!("app-icon-{name}"), egui::ColorImage::from_rgba_unmultiplied([w,h], &pixels), egui::TextureOptions::LINEAR));
                            if let Some(p) = self.packages.iter_mut().find(|p| p.name == name) {
                                if !p.matched { p.reason = core::name_reason(&p.name, Some(&info.label)); }
                            }
                            let note = format!("{} Categoria por nome atualizada.", info.note);
                            self.metadata.insert(name, AppMetadata { label: info.label, texture, note: info.note });
                            self.message(note);
                        }
                        Ok(Reply::Browser(target)) => { self.browser_confirm = Some(target); self.message("Navegador identificado. Revise a confirmação.".into()); }
                        Ok(Reply::Cleanup(plan)) => { self.cleanup_confirm = Some(plan); self.cleanup_checked = false; self.message("Prévia de limpeza pronta. Revise os arquivos antes de confirmar.".into()); }
                        Ok(Reply::Message(m)) => self.message(m),
                    }
                }
                Err(mpsc::TryRecvError::Disconnected) => { self.receiver = None; self.message("Trabalho interrompido. Analise novamente.".into()); }
                Err(mpsc::TryRecvError::Empty) => {}
            }
        }
        let busy = self.receiver.is_some();
        if busy { ctx.request_repaint_after(Duration::from_millis(100)); }
        let modal = self.pending.is_some() || self.dns_confirm.is_some() || self.animations_confirm.is_some() || self.browser_confirm.is_some() || self.cleanup_confirm.is_some();
        egui::TopBottomPanel::top("header").frame(egui::Frame::new().fill(colors.card).inner_margin(18)).show(ctx, |ui| {
            ui.horizontal(|ui| {
                avatar(ui, "Guard", None, 40.0);
                ui.vertical(|ui| { ui.label(RichText::new("AdwareGuard").size(24.0).strong()); ui.label(RichText::new("CONTROLE E REVISÃO DO SEU ANDROID").size(10.0).color(colors.muted)); });
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.add_enabled_ui(!modal, |ui| {
                        if theme_toggle(ui, &mut self.dark) {
                            apply_theme(ctx, self.dark); ctx.request_repaint();
                        }
                    });
                    ui.label(RichText::new("v0.7.0").color(colors.muted));
                    ui.label(RichText::new(if self.selected.is_empty() { "Aguardando conexão" } else { "Dispositivo selecionado" }).color(if self.selected.is_empty() { colors.muted } else { colors.accent }));
                });
            });
        });
        egui::TopBottomPanel::bottom("status").frame(egui::Frame::new().fill(colors.card).inner_margin(12)).show(ctx, |ui| {
            ui.horizontal(|ui| { if busy { ui.spinner(); } else { ui.colored_label(colors.accent, "•"); }
                ui.add(egui::Label::new(RichText::new(&self.status).small()).wrap()); });
            egui::CollapsingHeader::new("Histórico desta sessão").show(ui, |ui| {
                egui::ScrollArea::vertical().id_salt("history").max_height(110.0).show(ui, |ui| { for l in &self.log { ui.label(l); } });
            });
        });
        egui::SidePanel::left("sidebar").exact_width(280.0).resizable(false)
            .frame(egui::Frame::new().fill(colors.sidebar).inner_margin(18)).show(ctx, |ui| {
                egui::ScrollArea::vertical().id_salt("sidebar-scroll").show(ui, |ui| { ui.add_enabled_ui(!busy && !modal, |ui| self.sidebar(ui)); });
            });
        egui::CentralPanel::default().frame(egui::Frame::new().fill(colors.bg).inner_margin(22)).show(ctx, |ui| {
            ui.add_enabled_ui(!busy && !modal, |ui| {
                ui.horizontal(|ui| {
                    ui.vertical(|ui| { ui.heading("Visão geral"); ui.label(RichText::new("Revise cada aplicativo antes de tomar uma decisão.").color(colors.muted)); });
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if button(ui, "Mostrar resumo", false).clicked() { self.message(self.summary()); }
                    });
                });
                ui.add_space(18.0);
                let active = self.packages.iter().filter(|p| !p.removed).count();
                let listed = self.packages.iter().filter(|p| risk(p) == 1).count();
                let suspected = self.packages.iter().filter(|p| risk(p) == 2).count();
                ui.columns(4, |cols| {
                    metric(&mut cols[0], "APPS LISTADOS", active, colors.text);
                    metric(&mut cols[1], "LISTA LOCAL", listed, colors.danger);
                    metric(&mut cols[2], "SUSPEITAS POR NOME", suspected, colors.warning);
                    metric(&mut cols[3], "REMOÇÕES VERIFICADAS", self.removed, colors.accent);
                });
                ui.add_space(14.0);
                if self.scanned { ui.label(RichText::new(self.summary()).small().color(colors.muted)); }
                ui.add_space(16.0); ui.heading("Aplicativos"); ui.add_space(10.0);
                ui.horizontal(|ui| {
                    ui.add(egui::TextEdit::singleline(&mut self.filter).hint_text("Buscar por nome ou pacote…").desired_width(270.0));
                    egui::ComboBox::from_id_salt("app-kind").selected_text(["Todos os tipos", "Terceiros", "Sistema"][self.app_kind]).show_ui(ui, |ui| {
                        for (i,label) in ["Todos os tipos", "Terceiros", "Sistema"].iter().enumerate() { ui.selectable_value(&mut self.app_kind, i, *label); }
                    });
                });
                ui.add_space(8.0);
                ui.horizontal_wrapped(|ui| {
                    for (i,label) in RISK_LABELS.iter().enumerate() {
                        let count = self.packages.iter().filter(|p| i == 0 || risk(p) == i).count();
                        ui.selectable_value(&mut self.category, i, format!("{label} ({count})"));
                    }
                });
                ui.label(RichText::new(risk_description(self.category)).small().color(colors.muted));
                ui.label(RichText::new("Nome exibido: analisado após Carregar nome e ícone (AAPT2). Sem isso, a verificação usa o identificador do pacote.").small().color(colors.muted));
                ui.add_space(12.0);
                if !self.scanned { panel(ui).show(ui, |ui| { ui.add_space(22.0); ui.heading(if busy { "Análise em andamento" } else { "Tudo pronto para começar" });
                    ui.label("Selecione um dispositivo na lateral e clique em Analisar aplicativos."); ui.add_space(22.0); }); }
                else {
                    ui.horizontal_wrapped(|ui| {
                        ui.strong(format!("{} aplicativo(s) selecionado(s)", self.selected_apps.len()));
                        if ui.add_enabled(!self.selected_apps.is_empty(), egui::Button::new("Revisar e remover selecionados")).clicked() {
                            let apps: Vec<_> = self.packages.iter().filter(|p| self.selected_apps.contains(&p.name) && !p.system && !p.removed).cloned().collect();
                            if !apps.is_empty() { self.pending = Some(apps); self.confirmed = false; }
                        }
                        if ui.button("Limpar seleção").clicked() { self.selected_apps.clear(); }
                    });
                    ui.label(RichText::new("A seleção é mantida ao mudar os filtros; a confirmação mostra a lista completa. Selecionar não comprova que um app seja malicioso.").small().color(colors.muted));
                    let needle = self.filter.to_lowercase();
                    let mut request_remove = None; let mut request_icon = None;
                    let mut matches: Vec<_> = self.packages.iter().filter(|p| {
                        let search = p.name.to_lowercase().contains(&needle) || self.metadata.get(&p.name).is_some_and(|m| m.label.to_lowercase().contains(&needle));
                        search && (self.category == 0 || risk(p) == self.category) && match self.app_kind { 1 => !p.system, 2 => p.system, _ => true }
                    }).collect();
                    matches.sort_by_key(|p| (risk(p), p.name.clone()));
                    if matches.is_empty() { ui.label("Nenhum aplicativo corresponde a este filtro."); }
                    egui::ScrollArea::vertical().id_salt("apps").auto_shrink([false,false]).show(ui, |ui| {
                        let mut previous_risk = 0;
                        for p in matches {
                            let current_risk = risk(p);
                            if current_risk != previous_risk {
                                let count = self.packages.iter().filter(|p| risk(p) == current_risk && (self.app_kind == 0 || (self.app_kind == 1 && !p.system) || (self.app_kind == 2 && p.system)) && (p.name.to_lowercase().contains(&needle) || self.metadata.get(&p.name).is_some_and(|m| m.label.to_lowercase().contains(&needle)))).count();
                                ui.add_space(10.0);
                                ui.label(RichText::new(format!("{} · {}", RISK_LABELS[current_risk], count)).strong().size(16.0));
                                ui.label(RichText::new(risk_description(current_risk)).small().color(colors.muted));
                                ui.add_space(6.0); previous_risk = current_risk;
                            }
                            let meta = self.metadata.get(&p.name);
                            panel(ui).show(ui, |ui| {
                                ui.set_min_width((ui.available_width()-2.0).max(0.0));
                                ui.horizontal(|ui| {
                                    let mut checked = self.selected_apps.contains(&p.name);
                                    let selection = ui.add_enabled(!p.system && !p.removed, egui::Checkbox::without_text(&mut checked));
                                    if selection.changed() {
                                        if checked { self.selected_apps.insert(p.name.clone()); } else { self.selected_apps.remove(&p.name); }
                                    }
                                    selection.on_hover_text(if p.system { "Apps do sistema são protegidos" } else if p.removed { "App já removido" } else { "Selecionar para revisão de remoção" });
                                    avatar(ui, &p.name, meta.and_then(|m|m.texture.as_ref()), 44.0);
                                    ui.add_space(8.0);
                                    ui.vertical(|ui| {
                                        ui.label(RichText::new(meta.map(|m| m.label.as_str()).unwrap_or(&p.name)).size(15.0).strong());
                                        if meta.is_some() { ui.label(RichText::new(&p.name).small().color(colors.muted)); }
                                        if p.system { ui.label(RichText::new("SISTEMA · PROTEGIDO").size(10.0).color(colors.muted)); }
                                        let (tag,color) = match risk(p) {
                                            1 => ("CORRESPONDÊNCIA NA LISTA LOCAL", colors.danger),
                                            2 => ("SUSPEITA POR NOME · SINAL FRACO", colors.warning),
                                            4 => ("REMOVIDO", colors.accent),
                                            _ => ("SEM INDÍCIOS PELAS REGRAS", colors.muted),
                                        };
                                        ui.label(RichText::new(tag).size(10.0).color(color));
                                    });
                                });
                                ui.add_space(6.0); ui.label(RichText::new(&p.reason).small().color(colors.muted));
                                ui.horizontal(|ui| {
                                    if ui.add_enabled(!p.removed, egui::Button::new(if meta.is_some() { "Atualizar ícone" } else { "Carregar nome e ícone" })).on_hover_text("Lê o APK instalado usando AAPT2, sem instalar nada no celular.").clicked() { request_icon = Some(p.name.clone()); }
                                    if let Some(m) = meta { ui.label(RichText::new("ⓘ").color(colors.muted)).on_hover_text(&m.note); }
                                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                        if ui.add_enabled(!p.system && !p.removed, egui::Button::new(RichText::new("Revisar remoção").color(colors.danger))).clicked() { request_remove = Some(p.clone()); }
                                    });
                                });
                            });
                            ui.add_space(8.0);
                        }
                    });
                    if let Some(p) = request_remove { self.pending = Some(vec![p]); self.confirmed = false; }
                    if let Some(name) = request_icon { let adb = self.adb(); let aapt = PathBuf::from(self.aapt.trim()); let user = self.user.clone();
                        self.start(move || icons::load(&adb, aapt, user, name.clone()).map(|m|Reply::Metadata(name,m))); }
                }
            });
        });
        if modal {
            // Dim and intercept all clicks outside the confirmation dialog.
            egui::Area::new(egui::Id::new("modal-backdrop")).order(egui::Order::Middle).fixed_pos(egui::Pos2::ZERO).show(ctx, |ui| {
                let rect = ctx.screen_rect(); ui.allocate_rect(rect, egui::Sense::click()); ui.painter().rect_filled(rect, 0, Color32::from_black_alpha(155));
            });
        }
        if let Some(apps) = self.pending.clone() {
            egui::Window::new("Confirmar remoção dos aplicativos").id(egui::Id::new("remove-confirm")).order(egui::Order::Foreground)
                .anchor(egui::Align2::CENTER_CENTER, Vec2::ZERO).collapsible(false).resizable(false).default_width(560.0).show(ctx, |ui| {
                    ui.add_space(8.0); ui.heading(format!("Deseja remover {} aplicativo(s)?", apps.len()));
                    ui.label(format!("Dispositivo: {} · Usuário Android: {}", self.selected, self.user));
                    ui.add_space(10.0);
                    egui::ScrollArea::vertical().id_salt("batch-review").max_height(240.0).show(ui, |ui| {
                        for p in &apps {
                            ui.group(|ui| {
                                ui.horizontal(|ui| {
                                    avatar(ui, &p.name, self.metadata.get(&p.name).and_then(|m|m.texture.as_ref()), 32.0);
                                    ui.vertical(|ui| {
                                        ui.strong(self.metadata.get(&p.name).map(|m|m.label.as_str()).unwrap_or(&p.name));
                                        ui.label(RichText::new(&p.name).small().color(colors.muted));
                                    });
                                });
                                ui.label(RichText::new(&p.reason).small());
                            });
                        }
                    });
                    ui.add_space(10.0);
                    panel(ui).show(ui, |ui| { ui.colored_label(colors.danger, "A remoção pode apagar os dados desses aplicativos.");
                        ui.label("Faça backup e confirme a origem. Alertas por nome não comprovam infecção. Outros perfis podem manter os apps instalados."); });
                    ui.add_space(12.0);
                    ui.checkbox(&mut self.confirmed, "Entendo e desejo remover todos os aplicativos desta lista.");
                    ui.add_space(12.0);
                    ui.horizontal(|ui| {
                        if button(ui, "Cancelar", false).clicked() { self.pending = None; self.confirmed = false; }
                        ui.add_enabled_ui(self.confirmed && !busy && !apps.is_empty(), |ui| {
                            if ui.add(egui::Button::new(RichText::new("Confirmar e remover").strong().color(if self.dark { colors.bg } else { Color32::WHITE })).fill(colors.danger).min_size(Vec2::new(180.0,38.0)).corner_radius(8)).clicked() {
                                let adb = self.adb(); let user = self.user.clone(); let count = apps.len(); self.pending = None; self.confirmed = false;
                                self.start(move || {
                                    let results = apps.into_iter().map(|p| {
                                        let result = if p.system || p.removed { Err("App protegido ou já removido".into()) } else { adb.remove(&user, &p.name) };
                                        (p, result)
                                    }).collect();
                                    Ok(Reply::Batch(results))
                                });
                                self.status = format!("Removendo {count} aplicativo(s), um por vez. Aguarde o resumo final.");
                            }
                        });
                    });
                });
        }
        if let Some(disabled) = self.animations_confirm {
            egui::Window::new("Confirmar ajuste de animações").order(egui::Order::Foreground).anchor(egui::Align2::CENTER_CENTER, Vec2::ZERO)
                .collapsible(false).resizable(false).default_width(470.0).show(ctx, |ui| {
                    ui.heading(if disabled { "Ativar redução/remoção de animações?" } else { "Usar animações padrão (1×)?" });
                    ui.label(format!("Aparelho: {}", self.selected));
                    ui.label("Serão ajustadas as três escalas de animação e, quando expostos, os ajustes extras de redução/remoção do fabricante. No Android padrão, as três escalas também controlam a opção de acessibilidade.");
                    ui.label(if disabled { "Alguns aplicativos podem manter efeitos próprios. Se o fabricante bloquear o comando, ajuste as três escalas manualmente em Opções do desenvolvedor." }
                        else { "Aplica 1× nas três escalas e desliga os ajustes extras expostos; não recupera personalizações anteriores." });
                    ui.add_space(12.0);
                    ui.horizontal(|ui| {
                        if button(ui, "Cancelar", false).clicked() { self.animations_confirm = None; }
                        if button(ui, "Confirmar ajuste", true).clicked() {
                            let adb = self.adb(); self.animations_confirm = None;
                            self.start(move || adb.accessibility_motion(disabled).map(Reply::Message));
                        }
                    });
                });
        }
        if let Some(target) = self.browser_confirm.clone() {
            egui::Window::new("Cache do navegador").order(egui::Order::Foreground).anchor(egui::Align2::CENTER_CENTER, Vec2::ZERO)
                .collapsible(false).resizable(false).default_width(500.0).show(ctx, |ui| {
                    ui.heading(if target.cache_only_supported { "Limpar somente o cache?" } else { "Limpeza manual no celular" });
                    ui.label(format!("Aparelho: {} · Usuário: {}",self.selected,target.user));
                    ui.strong(&target.package);
                    if target.explicit_choice { ui.label("Navegador escolhido por você; não precisa ser o padrão."); }
                    else if !target.default_confirmed { ui.colored_label(colors.warning,"Padrão não identificado. Chrome foi encontrado como alternativa; confirme se é o navegador que deseja limpar."); }
                    if target.cache_only_supported {
                        ui.label("O aparelho anuncia suporte à limpeza somente de cache. O programa não solicita exclusão de cookies, histórico, senhas ou outros dados. O cache pode ser recriado durante o uso.");
                    } else {
                        ui.label("Este aparelho não confirmou suporte seguro a limpar apenas cache por ADB. Vamos abrir os detalhes do navegador; toque em Armazenamento → Limpar cache. Não escolha Limpar dados/armazenamento.");
                    }
                    ui.add_space(12.0); ui.horizontal(|ui| {
                        if button(ui,"Cancelar",false).clicked() { self.browser_confirm = None; }
                        if button(ui,if target.cache_only_supported { "Confirmar limpeza de cache" } else { "Abrir detalhes no celular" },true).clicked() {
                            let adb = self.adb(); self.browser_confirm = None;
                            self.start(move || if target.cache_only_supported { adb.clear_browser_cache(&target).map(Reply::Message) } else { adb.open_browser_storage(&target).map(Reply::Message) });
                        }
                    });
                });
        }
        if let Some(plan) = self.cleanup_confirm.clone() {
            egui::Window::new("Revisar limpeza do aparelho").order(egui::Order::Foreground).anchor(egui::Align2::CENTER_CENTER, Vec2::ZERO)
                .collapsible(false).resizable(false).default_width(600.0).show(ctx, |ui| {
                    ui.heading(format!("{} miniatura(s) elegível(is)",plan.paths.len()));
                    ui.label(format!("Aparelho: {} · Usuário: {}",self.selected,plan.user));
                    ui.label("Somente arquivos diretos das pastas .thumbnails de DCIM/Pictures. Fotos originais fora dessas pastas, Downloads, WhatsApp e caches de apps ficam fora da limpeza. A galeria pode recriar essas miniaturas.");
                    egui::ScrollArea::vertical().id_salt("cleanup-preview").max_height(230.0).show(ui, |ui| {
                        for path in &plan.paths { ui.label(RichText::new(path).small()); }
                        for note in &plan.notes { ui.colored_label(colors.warning,note); }
                    });
                    if plan.paths.is_empty() { ui.label("Nenhuma miniatura elegível encontrada ou acessível. Isso não significa que o aparelho não tenha cache."); }
                    ui.add_space(10.0);
                    ui.checkbox(&mut self.cleanup_checked,"Revisei a lista e desejo apagar somente esses arquivos de miniaturas.");
                    ui.horizontal(|ui| {
                        if button(ui,"Cancelar",false).clicked() { self.cleanup_confirm = None; self.cleanup_checked = false; }
                        ui.add_enabled_ui(self.cleanup_checked && !plan.paths.is_empty(),|ui| {
                            if button(ui,"Confirmar limpeza",true).clicked() {
                                let adb = self.adb(); self.cleanup_confirm = None; self.cleanup_checked = false;
                                self.start(move || adb.clean_device_cache(&plan).map(Reply::Message));
                            }
                        });
                    });
                });
        }
        if let Some(auto) = self.dns_confirm {
            egui::Window::new("Confirmar mudança do DNS").order(egui::Order::Foreground).anchor(egui::Align2::CENTER_CENTER, Vec2::ZERO)
                .collapsible(false).resizable(false).default_width(460.0).show(ctx, |ui| {
                    ui.label(if auto { "Alterar DNS privado para Automático?" } else { "Usar dns.adguard-dns.com? As consultas DNS usarão esse provedor." });
                    ui.label(format!("Aparelho: {}. A configuração pode afetar todos os usuários do Android.", self.selected));
                    ui.label("Se a rede parar de funcionar, selecione Automático no menu DNS privado do celular.");
                    ui.add_space(12.0); ui.horizontal(|ui| {
                        if button(ui, "Cancelar", false).clicked() { self.dns_confirm = None; }
                        if button(ui, "Aplicar configuração", true).clicked() { let adb = self.adb(); self.dns_confirm = None; self.start(move || adb.dns(auto).map(Reply::Message)); }
                    });
                });
        }
    }
}
fn window_icon() -> egui::IconData {
    let mut rgba = vec![0; 32*32*4];
    for y in 0..32usize { for x in 0..32usize {
        let shield = (5..27).contains(&x) && (3..23).contains(&y) || ((23..30).contains(&y) && x.abs_diff(16) < 30-y);
        let check = ((10..16).contains(&x) && y.abs_diff(x+7) < 2) || ((15..24).contains(&x) && y.abs_diff(37-x) < 2);
        let c = if check && shield { [13,20,33,255] } else if shield { [57,213,177,255] } else { [0,0,0,0] };
        rgba[(y*32+x)*4..(y*32+x+1)*4].copy_from_slice(&c);
    } }
    egui::IconData { rgba, width: 32, height: 32 }
}
fn main() -> eframe::Result<()> {
    let options = eframe::NativeOptions { viewport: egui::ViewportBuilder::default().with_inner_size([1220.0, 860.0]).with_min_inner_size([1080.0, 720.0]).with_icon(window_icon()), ..Default::default() };
    eframe::run_native("AdwareGuard", options, Box::new(|cc| {
        apply_theme(&cc.egui_ctx, false);
        Ok(Box::new(App::default()))
    }))
}

#[cfg(test)]
mod presentation_tests {
    use super::*;
    fn package() -> Package { Package { name: "com.test.app".into(), system: true, reason: "Nome contém termo".into(), matched: false, removed: false } }
    #[test]
    fn partial_batch_counts_only_verified_successes_and_keeps_failures_selected() {
        let mut first = package(); first.system = false; first.matched = true;
        let mut second = first.clone(); second.name = "com.test.second".into(); second.matched = false;
        let mut app = App::default();
        assert!(!app.dark);
        app.scanned = true; app.packages = vec![first.clone(), second.clone()];
        app.selected_apps = [first.name.clone(), second.name.clone()].into_iter().collect();
        app.apply_batch(vec![(first.clone(), Ok(())), (second.clone(), Err("ADB recusou".into()))]);
        assert_eq!(app.removed, 1); assert_eq!(app.matched_removed, 1);
        assert!(app.packages[0].removed); assert!(!app.packages[1].removed);
        assert!(!app.selected_apps.contains(&first.name)); assert!(app.selected_apps.contains(&second.name));
        assert!(app.log.iter().any(|line| line.contains("ADB recusou")));
    }
    #[test]
    fn risk_groups_do_not_overlap_or_hide_system_alerts() {
        let mut p = package();
        assert_eq!(risk(&p), 2); // System protection is independent of risk.
        p.matched = true; assert_eq!(risk(&p), 1);
        p.removed = true; assert_eq!(risk(&p), 4);
        p.removed = false; p.matched = false; p.reason = "Sem indício pelas regras disponíveis".into();
        assert_eq!(risk(&p), 3);
    }
}
