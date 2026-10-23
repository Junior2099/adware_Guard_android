#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
mod core;
mod icons;
use core::{Adb, Device, Package};
use eframe::egui::{self, Color32, RichText, Vec2};
use std::{collections::HashMap, path::PathBuf, sync::mpsc::{self, Receiver}, time::Duration};
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
        2 => "Termo suspeito no nome do pacote. Sinal fraco, sujeito a falsos positivos.",
        3 => "Nenhum indício pelas regras disponíveis. Isso não significa que o app seja seguro.",
        4 => "Remoções verificadas nesta análise, para o usuário Android selecionado.",
        _ => "Categorias de análise separadas do tipo de app. Pacotes do sistema continuam protegidos.",
    }
}
enum Reply { Devices(Vec<Device>), Scan(String, Vec<Package>), Removed(String, bool), Message(String), Metadata(String, icons::Metadata) }
struct AppMetadata { label: String, texture: Option<egui::TextureHandle>, note: String }
struct App {
    dark: bool, app_kind: usize, path: String, aapt: String, devices: Vec<Device>, selected: String, user: String,
    packages: Vec<Package>, filter: String, category: usize, status: String, log: Vec<String>,
    receiver: Option<Receiver<core::Result<Reply>>>, pending: Option<Package>, confirmed: bool,
    dns_confirm: Option<bool>, scanned: bool, removed: usize, matched_removed: usize,
    metadata: HashMap<String, AppMetadata>,
}
impl Default for App {
    fn default() -> Self { Self {
        dark: true, app_kind: 0, path: core::root().join("tools/platform-tools/adb.exe").display().to_string(),
        aapt: core::root().join("tools/aapt2/aapt2.exe").display().to_string(),
        devices: vec![], selected: String::new(), user: String::new(), packages: vec![], filter: String::new(), category: 0,
        status: "Conecte o celular por USB para começar.".into(), log: vec![], receiver: None,
        pending: None, confirmed: false, dns_confirm: None, scanned: false, removed: 0, matched_removed: 0, metadata: HashMap::new(),
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
        if before != self.selected { self.packages.clear(); self.metadata.clear(); self.scanned = false; self.removed = 0; self.matched_removed = 0; }
        ui.add_space(12.0);
        ui.add_enabled_ui(!self.selected.is_empty(), |ui| {
            if button(ui, "2. Analisar aplicativos", true).clicked() {
                let adb = self.adb(); self.scanned = false; self.packages.clear();
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
                            self.metadata.clear(); self.devices = d; self.packages.clear(); self.scanned = false;
                            self.selected = self.devices.iter().find(|d| d.state == "device").map(|d| d.serial.clone()).unwrap_or_default();
                            self.removed = 0; self.matched_removed = 0;
                            self.message("Detecção concluída. Selecione um aparelho autorizado e analise.".into());
                        }
                        Ok(Reply::Scan(user, packages)) => { self.user = user; self.packages = packages; self.scanned = true;
                            self.message(format!("Análise concluída: {} pacotes. {}", self.packages.len(), self.summary())); }
                        Ok(Reply::Removed(name, matched)) => {
                            if let Some(p) = self.packages.iter_mut().find(|p| p.name == name) { p.removed = true; }
                            self.removed += 1; if matched { self.matched_removed += 1; }
                            self.message(format!("{name}: remoção verificada para o usuário {}. {}", self.user, self.summary()));
                        }
                        Ok(Reply::Metadata(name, info)) => {
                            let texture = info.pixels.map(|(w,h,pixels)| ctx.load_texture(
                                format!("app-icon-{name}"), egui::ColorImage::from_rgba_unmultiplied([w,h], &pixels), egui::TextureOptions::LINEAR));
                            let note = info.note.clone();
                            self.metadata.insert(name, AppMetadata { label: info.label, texture, note: info.note });
                            self.message(note);
                        }
                        Ok(Reply::Message(m)) => self.message(m),
                    }
                }
                Err(mpsc::TryRecvError::Disconnected) => { self.receiver = None; self.message("Trabalho interrompido. Analise novamente.".into()); }
                Err(mpsc::TryRecvError::Empty) => {}
            }
        }
        let busy = self.receiver.is_some();
        if busy { ctx.request_repaint_after(Duration::from_millis(100)); }
        let modal = self.pending.is_some() || self.dns_confirm.is_some();
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
                    ui.label(RichText::new("v0.3.0").color(colors.muted));
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
                ui.add_space(12.0);
                if !self.scanned { panel(ui).show(ui, |ui| { ui.add_space(22.0); ui.heading(if busy { "Análise em andamento" } else { "Tudo pronto para começar" });
                    ui.label("Selecione um dispositivo na lateral e clique em Analisar aplicativos."); ui.add_space(22.0); }); }
                else {
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
                    if let Some(p) = request_remove { self.pending = Some(p); self.confirmed = false; }
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
        if let Some(p) = self.pending.clone() {
            egui::Window::new("Confirmar remoção").id(egui::Id::new("remove-confirm")).order(egui::Order::Foreground)
                .anchor(egui::Align2::CENTER_CENTER, Vec2::ZERO).collapsible(false).resizable(false).default_width(480.0).show(ctx, |ui| {
                    ui.add_space(8.0);
                    ui.horizontal(|ui| { avatar(ui, &p.name, self.metadata.get(&p.name).and_then(|m|m.texture.as_ref()), 48.0);
                        ui.vertical(|ui| { ui.strong(self.metadata.get(&p.name).map(|m|m.label.as_str()).unwrap_or(&p.name)); ui.label(RichText::new(&p.name).small().color(colors.muted)); }); });
                    ui.add_space(14.0); ui.label(format!("Dispositivo: {} · Usuário Android: {}", self.selected, self.user));
                    ui.label(&p.reason); ui.add_space(12.0);
                    panel(ui).show(ui, |ui| { ui.colored_label(colors.danger, "A remoção pode apagar os dados deste aplicativo.");
                        ui.label("Faça backup e confirme a origem. Outros perfis do Android podem manter o app instalado."); });
                    ui.add_space(14.0);
                    ui.checkbox(&mut self.confirmed, "Entendo e desejo remover este aplicativo.");
                    ui.add_space(14.0);
                    ui.horizontal(|ui| {
                        if button(ui, "Cancelar", false).clicked() { self.pending = None; self.confirmed = false; }
                        ui.add_enabled_ui(self.confirmed && !busy, |ui| {
                            if ui.add(egui::Button::new(RichText::new("Confirmar e remover").strong().color(if self.dark { colors.bg } else { Color32::WHITE })).fill(colors.danger).min_size(Vec2::new(180.0,38.0)).corner_radius(8)).clicked() {
                                let adb = self.adb(); let user = self.user.clone(); self.pending = None; self.confirmed = false;
                                self.start(move || { adb.remove(&user, &p.name)?; Ok(Reply::Removed(p.name, p.matched)) });
                            }
                        });
                    }); ui.add_space(6.0);
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
        apply_theme(&cc.egui_ctx, true);
        Ok(Box::new(App::default()))
    }))
}

#[cfg(test)]
mod presentation_tests {
    use super::*;
    fn package() -> Package { Package { name: "com.test.app".into(), system: true, reason: "Nome contém termo".into(), matched: false, removed: false } }
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
