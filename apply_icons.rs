use std::fs;

fn main() {
    let mut content = fs::read_to_string("src/app.rs").unwrap().replace("\r\n", "\n");

    // 1. In setup_visuals, add egui_phosphor
    let font_setup_target = r#"        let mut fonts = egui::FontDefinitions::default();
        fonts.font_data.insert(
            "JetBrains Mono".to_owned(),
            egui::FontData::from_static(include_bytes!("../assets/JetBrainsMono-Regular.ttf")),
        );"#;
    let font_setup_new = r#"        let mut fonts = egui::FontDefinitions::default();
        egui_phosphor::add_to_fonts(&mut fonts, egui_phosphor::Variant::Regular);
        fonts.font_data.insert(
            "JetBrains Mono".to_owned(),
            egui::FontData::from_static(include_bytes!("../assets/JetBrainsMono-Regular.ttf")),
        );"#;
    if !content.contains("egui_phosphor::add_to_fonts") {
        content = content.replace(font_setup_target, font_setup_new);
    }

    // Since egui_phosphor adds to fallback lists, we just need to ensure the Unicode constants map properly.
    // Replace X, O, _ with Phosphor icons
    let controls_target = r#"if ui.add(egui::Button::new(RichText::new(" X ").color(ctrl_col).size(14.0)).frame(false)).clicked() {
                            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                        }
                        ui.add_space(4.0);
                        if ui.add(egui::Button::new(RichText::new(" O ").color(ctrl_col).size(14.0)).frame(false)).clicked() {
                            let is_max = ctx.input(|i| i.viewport().maximized.unwrap_or(false));
                            ctx.send_viewport_cmd(egui::ViewportCommand::Maximized(!is_max));
                        }
                        ui.add_space(4.0);
                        if ui.add(egui::Button::new(RichText::new(" _ ").color(ctrl_col).size(14.0)).frame(false)).clicked() {
                            ctx.send_viewport_cmd(egui::ViewportCommand::Minimized(true));
                        }"#;
    let controls_new = r#"if ui.add(egui::Button::new(RichText::new(egui_phosphor::regular::X).color(ctrl_col).size(16.0)).frame(false)).clicked() {
                            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                        }
                        ui.add_space(4.0);
                        if ui.add(egui::Button::new(RichText::new(egui_phosphor::regular::SQUARE).color(ctrl_col).size(16.0)).frame(false)).clicked() {
                            let is_max = ctx.input(|i| i.viewport().maximized.unwrap_or(false));
                            ctx.send_viewport_cmd(egui::ViewportCommand::Maximized(!is_max));
                        }
                        ui.add_space(4.0);
                        if ui.add(egui::Button::new(RichText::new(egui_phosphor::regular::MINUS).color(ctrl_col).size(16.0)).frame(false)).clicked() {
                            ctx.send_viewport_cmd(egui::ViewportCommand::Minimized(true));
                        }"#;
    content = content.replace(controls_target, controls_new);

    // Replace GEAR, SEARCH, TERMINAL
    let icons_target = r#"if ui.add(egui::Button::new(RichText::new("⚙").color(TEXT_PRIMARY).size(18.0)).frame(false)).clicked() {
                            self.current_screen = AppScreen::Settings;
                        }
                        ui.add_space(20.0);
                        if ui.add(egui::Button::new(RichText::new(">_").color(TEXT_PRIMARY).size(18.0)).frame(false)).clicked() {}
                        
                        ui.add_space(24.0);
                        // Search Box
                        let (rect, _) = ui.allocate_exact_size(Vec2::new(260.0, 32.0), egui::Sense::hover());
                        ui.painter().rect_filled(rect, 4.0, BG_INPUT);
                        ui.painter().text(
                            rect.left_center() + Vec2::new(12.0, 0.0),
                            egui::Align2::LEFT_CENTER,
                            "🔍 Search commands...",
                            FontId::proportional(13.0),
                            TEXT_DIM
                        );"#;
    let icons_new = r#"if ui.add(egui::Button::new(RichText::new(egui_phosphor::regular::GEAR).color(TEXT_PRIMARY).size(18.0)).frame(false)).clicked() {
                            self.current_screen = AppScreen::Settings;
                        }
                        ui.add_space(20.0);
                        if ui.add(egui::Button::new(RichText::new(egui_phosphor::regular::TERMINAL_WINDOW).color(TEXT_PRIMARY).size(18.0)).frame(false)).clicked() {
                            self.current_screen = AppScreen::Editor;
                        }
                        
                        ui.add_space(24.0);
                        // Search Box
                        let (rect, _) = ui.allocate_exact_size(Vec2::new(260.0, 32.0), egui::Sense::hover());
                        ui.painter().rect_filled(rect, 4.0, BG_INPUT);
                        
                        let search_text = format!("{} Search commands...", egui_phosphor::regular::MAGNIFYING_GLASS);
                        ui.painter().text(
                            rect.left_center() + Vec2::new(12.0, 0.0),
                            egui::Align2::LEFT_CENTER,
                            search_text,
                            egui::FontId::proportional(13.0),
                            TEXT_DIM
                        );"#;
    content = content.replace(icons_target, icons_new);

    // Re-insert render_side_nav_bar into update! Wait, update used to have `self.render_footer(ctx);`.
    let update_target = r#"        if self.layout_mode == LayoutMode::Portrait {
             self.render_mobile_bottom_nav(ctx);
        } else {
             self.render_footer(ctx);
        }"#;
    let update_new = r#"        if self.layout_mode == LayoutMode::Portrait {
             self.render_mobile_bottom_nav(ctx);
        } else {
             self.render_side_nav_bar(ctx);
             self.render_footer(ctx);
        }"#;
    if !content.contains("self.render_side_nav_bar(ctx);") {
        content = content.replace(update_target, update_new);
    }
    
    // Add render_side_nav_bar method to app.rs just before render_footer
    let footer_start = "fn render_footer(&mut self, ctx: &egui::Context) {";
    let sidebar_code = r#"    fn render_side_nav_bar(&mut self, ctx: &egui::Context) {
        egui::SidePanel::left("ultra_slim_rail")
            .exact_width(64.0)
            .frame(egui::Frame::none()
                .fill(BG_PANEL)
                .stroke(egui::Stroke::new(1.0, BG_SIDEBAR)))
            .show(ctx, |ui| {
                ui.vertical_centered(|ui| {
                    ui.add_space(16.0);
                    ui.label(RichText::new("VIBE").color(ACCENT).size(12.0).strong());
                    ui.add_space(4.0);
                    ui.label(RichText::new("V1.0.4").color(TEXT_DIM).size(9.0));
                    ui.add_space(32.0);
                    if ui.add(egui::Button::new(RichText::new(egui_phosphor::regular::FOLDER).color(TEXT_DIM).size(20.0)).frame(false)).clicked() {
                        self.current_screen = AppScreen::Editor;
                    }
                });
            });
    }

    "#;
    if !content.contains("fn render_side_nav_bar(&mut self") {
        content = content.replace(footer_start, &format!("{}{}", sidebar_code, footer_start));
    }

    fs::write("src/app.rs", content).unwrap();
    println!("Icons and Sidebar injected!");
}
