
impl VibingApp {
    fn render_top_app_bar(&mut self, ctx: &egui::Context) {
        egui::TopBottomPanel::top("top_app_bar")
            .exact_height(48.0)
            .frame(egui::Frame::none()
                .fill(Color32::from_rgba_premultiplied(19, 19, 19, 153)) // #131313/60
                .stroke(egui::Stroke::new(1.0, BORDER_COLOR)))
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.add_space(24.0);
                    ui.label(RichText::new("VibingIDE")
                        .color(ACCENT)
                        .size(18.0)
                        .strong());
                    
                    // Search bar mockup
                    ui.add_space(24.0);
                    let (rect, _) = ui.allocate_exact_size(Vec2::new(256.0, 24.0), egui::Sense::hover());
                    ui.painter().rect_filled(rect, 2.0, BG_INPUT);
                    ui.painter().rect_stroke(rect, 2.0, egui::Stroke::new(1.0, BORDER_COLOR));
                    ui.painter().text(
                        rect.left_center() + Vec2::new(8.0, 0.0),
                        egui::Align2::LEFT_CENTER,
                        "Search components...",
                        FontId::proportional(12.0),
                        TEXT_DIM
                    );
                    
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.add_space(24.0);
                        if ui.add(egui::Button::new(RichText::new(" ☰ ").color(TEXT_PRIMARY).size(18.0)).frame(false)).clicked() {}
                        if ui.add(egui::Button::new(RichText::new(" ⚡ ").color(TEXT_PRIMARY).size(18.0)).frame(false)).clicked() {}
                        if ui.add(egui::Button::new(RichText::new(" >_ ").color(TEXT_PRIMARY).size(18.0)).frame(false)).clicked() {}
                    });
                });
            });
    }

    fn render_side_nav_bar(&mut self, ctx: &egui::Context) {
        egui::SidePanel::left("side_nav_bar")
            .exact_width(64.0)
            .frame(egui::Frame::none()
                .fill(BG_PANEL)
                .stroke(egui::Stroke::new(1.0, BG_SIDEBAR)))
            .show(ctx, |ui| {
                ui.vertical_centered(|ui| {
                    ui.add_space(24.0);
                    ui.label(RichText::new("V").color(ACCENT).size(24.0).strong());
                    ui.add_space(32.0);
                    
                    if ui.add(egui::Button::new(RichText::new("🗀").size(24.0).color(if self.current_screen == AppScreen::Editor { ACCENT } else { TEXT_DIM })).frame(false)).clicked() {
                        self.current_screen = AppScreen::Editor;
                    }
                    ui.add_space(16.0);
                    
                    if ui.add(egui::Button::new(RichText::new("🤖").size(24.0).color(if self.current_screen == AppScreen::Agents { ACCENT } else { TEXT_DIM })).frame(false)).clicked() {
                        self.current_screen = AppScreen::Agents;
                    }
                    ui.add_space(16.0);

                    if ui.add(egui::Button::new(RichText::new("⚙").size(24.0).color(if self.current_screen == AppScreen::Settings { ACCENT } else { TEXT_DIM })).frame(false)).clicked() {
                        self.current_screen = AppScreen::Settings;
                    }
                });
            });
    }

    fn render_mobile_bottom_nav(&mut self, ctx: &egui::Context) {
        egui::TopBottomPanel::bottom("mobile_bottom_nav")
            .exact_height(56.0)
            .frame(egui::Frame::none().fill(BG_PANEL).stroke(egui::Stroke::new(2.0, BG_SIDEBAR)))
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    let w = ui.available_width() / 3.0;
                    if ui.add_sized([w, 56.0], egui::Button::new("Editor").frame(false)).clicked() { self.current_screen = AppScreen::Editor; }
                    if ui.add_sized([w, 56.0], egui::Button::new("Agents").frame(false)).clicked() { self.current_screen = AppScreen::Agents; }
                    if ui.add_sized([w, 56.0], egui::Button::new("Settings").frame(false)).clicked() { self.current_screen = AppScreen::Settings; }
                });
            });
    }

    fn render_footer(&mut self, ctx: &egui::Context) {
        egui::TopBottomPanel::bottom("footer")
            .exact_height(24.0)
            .frame(egui::Frame::none()
                .fill(BG_PANEL)
                .stroke(egui::Stroke::new(1.0, BORDER_COLOR)))
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.add_space(16.0);
                    ui.label(RichText::new("main*").color(ACCENT).size(11.0));
                    ui.add_space(16.0);
                    ui.label(RichText::new("● Agent: Idle").color(TEXT_DIM).size(11.0));
                    
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.add_space(16.0);
                        ui.label(RichText::new("v1.0.4-stable").color(Color32::from_rgb(183, 234, 255)).size(11.0));
                        ui.add_space(16.0);
                        ui.label(RichText::new("UTF-8").color(TEXT_DIM).size(11.0));
                    });
                });
            });
    }

    fn render_screen_editor(&mut self, ui: &mut Ui, ctx: &egui::Context) {
        // Render 2 panels: Left file tree sidebar, right Editor + terminal split
        self.render_wide_sidebar(ctx);
        if self.panel_mgr.panels().is_empty() {
            self.render_empty_state(ui);
        } else {
            self.render_panels_wide(ui, ctx);
        }
    }

    fn render_screen_agents(&mut self, ui: &mut Ui, ctx: &egui::Context) {
        ui.vertical_centered(|ui| {
            ui.add_space(32.0);
            ui.label(RichText::new("Agent Command Center Screen Mockup").size(24.0).color(ACCENT));
            if self.panel_mgr.panels().is_empty() {
                self.render_empty_state(ui);
            } else {
                self.render_panels_wide(ui, ctx); 
            }
        });
    }

    fn render_screen_settings(&mut self, ui: &mut Ui, ctx: &egui::Context) {
        ui.vertical_centered(|ui| {
            ui.add_space(32.0);
            ui.label(RichText::new("Settings Configuration Desktop Mockup").size(24.0).color(TEXT_PRIMARY));
        });
    }
}
