use std::fs;

fn main() {
    let mut content = fs::read_to_string("src/app.rs").unwrap().replace("\r\n", "\n");

    // 1. Add setup_visuals fonts
    let font_setup_target = r#"        let mut fonts = egui::FontDefinitions::default();
        fonts.font_data.insert("#;
    let font_setup_new = r#"        let mut fonts = egui::FontDefinitions::default();
        egui_phosphor::add_to_fonts(&mut fonts, egui_phosphor::Variant::Regular);
        fonts.font_data.insert("#;
    if !content.contains("egui_phosphor::add_to_fonts") {
        content = content.replace(font_setup_target, font_setup_new);
    }

    // 2. Add Infrastructure to AppScreen
    let app_screen_target = "pub enum AppScreen {\n    Editor,\n    Agents,\n    Settings,\n}";
    let app_screen_new = "pub enum AppScreen {\n    Editor,\n    Agents,\n    Infrastructure,\n    Settings,\n}";
    if !content.contains("Infrastructure,") {
        content = content.replace(app_screen_target, app_screen_new);
        
        // Add to match
        let app_match_target = "match self.current_screen {\n                     AppScreen::Editor => self.render_screen_editor(ui, ctx),\n                     AppScreen::Agents => self.render_screen_agents(ui, ctx),\n                     AppScreen::Settings => self.render_screen_settings(ui, ctx),\n                 }";
        let app_match_new = "match self.current_screen {\n                     AppScreen::Editor => self.render_screen_editor(ui, ctx),\n                     AppScreen::Agents => self.render_screen_agents(ui, ctx),\n                     AppScreen::Settings => self.render_screen_settings(ui, ctx),\n                     AppScreen::Infrastructure => { ui.centered_and_justified(|ui| ui.label(RichText::new(\"Infrastructure Coming Soon\").color(TEXT_DIM))); }\n                 }";
        content = content.replace(app_match_target, app_match_new);
    }
    
    // 3. Update the self.update layout calling order
    let update_target = r#"        if self.layout_mode == LayoutMode::Portrait {
             self.render_mobile_bottom_nav(ctx);
        } else {
             self.render_side_nav_bar(ctx);
             self.render_footer(ctx);
        }
        self.render_top_app_bar(ctx);"#;
    let update_new = r#"        if self.layout_mode == LayoutMode::Portrait {
             self.render_mobile_bottom_nav(ctx);
        } else {
             self.render_side_nav_bar(ctx);
             self.render_footer(ctx);
             self.render_top_app_bar(ctx);
        }"#;
    content = content.replace(update_target, update_new);


    // 4. Overwrite render_top_app_bar and render_side_nav_bar completely.
    let start_idx = content.find("    fn render_top_app_bar(&mut self, ctx: &egui::Context) {").unwrap();
    let end_idx = content.find("    fn render_mobile_bottom_nav(&mut self, ctx: &egui::Context) {").unwrap();
    
    let new_bars = r#"    fn render_side_nav_bar(&mut self, ctx: &egui::Context) {
        egui::SidePanel::left("ultra_slim_rail")
            .exact_width(64.0)
            .frame(egui::Frame::none()
                .fill(BG_PANEL)
                .stroke(egui::Stroke::new(1.0, BG_SIDEBAR)))
            .show(ctx, |ui| {
                ui.vertical_centered(|ui| {
                    ui.add_space(20.0);
                    ui.label(RichText::new("VIBE").color(ACCENT).size(11.0).strong());
                    ui.add_space(2.0);
                    ui.label(RichText::new("V1.0.4").color(TEXT_DIM).size(9.0));
                    ui.add_space(42.0);
                    if ui.add(egui::Button::new(RichText::new(egui_phosphor::regular::FOLDER).size(22.0).color(if self.current_screen == AppScreen::Editor { ACCENT } else { TEXT_DIM })).frame(false)).clicked() {
                        self.current_screen = AppScreen::Editor;
                    }
                    ui.add_space(24.0);
                    if ui.add(egui::Button::new(RichText::new(egui_phosphor::regular::ROBOT).size(22.0).color(if self.current_screen == AppScreen::Agents { ACCENT } else { TEXT_DIM })).frame(false)).clicked() {
                        self.current_screen = AppScreen::Agents;
                    }
                });
            });
    }

    fn render_top_app_bar(&mut self, ctx: &egui::Context) {
        let top_response = egui::TopBottomPanel::top("top_app_bar")
            .exact_height(56.0)
            .frame(egui::Frame::none()
                .fill(Color32::from_rgb(19, 19, 19))
                .stroke(egui::Stroke::new(1.0, BORDER_COLOR)))
            .show(ctx, |ui| {
                ui.horizontal_centered(|ui| {
                    ui.add_space(24.0);
                    // VIBINGIDE
                    ui.label(RichText::new("VIBINGIDE")
                        .color(ACCENT)
                        .size(18.0)
                        .strong());
                    
                    ui.add_space(32.0); // gap before tabs
                    
                    // Tabs
                    let tabs = [
                        ("Command Center", AppScreen::Agents),
                        ("Editor", AppScreen::Editor),
                        ("Infrastructure", AppScreen::Infrastructure),
                    ];
                    
                    for (name, screen) in tabs {
                        let active = self.current_screen == screen;
                        let color = if active { ACCENT } else { TEXT_DIM };
                        let text = RichText::new(name).size(14.0).color(color);
                        
                        let response = ui.add_sized([0.0, 56.0], egui::Button::new(text).frame(false));
                        if response.clicked() {
                            self.current_screen = screen;
                        }
                        
                        if active {
                            let rect = response.rect;
                            ui.painter().rect_filled(
                                egui::Rect::from_min_max(
                                    egui::pos2(rect.left(), ui.max_rect().bottom() - 2.0),
                                    egui::pos2(rect.right(), ui.max_rect().bottom())
                                ),
                                0.0,
                                ACCENT
                            );
                        }
                        ui.add_space(16.0);
                    }
                    
                    // Right aligned section
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.add_space(16.0);
                        // Window controls (X, [], _)
                        let ctrl_col = Color32::from_rgb(0, 150, 100);
                        if ui.add(egui::Button::new(RichText::new(egui_phosphor::regular::X).color(ctrl_col).size(16.0)).frame(false)).clicked() {
                            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                        }
                        ui.add_space(8.0);
                        if ui.add(egui::Button::new(RichText::new(egui_phosphor::regular::SQUARE).color(ctrl_col).size(16.0)).frame(false)).clicked() {
                            let is_max = ctx.input(|i| i.viewport().maximized.unwrap_or(false));
                            ctx.send_viewport_cmd(egui::ViewportCommand::Maximized(!is_max));
                        }
                        ui.add_space(8.0);
                        if ui.add(egui::Button::new(RichText::new(egui_phosphor::regular::MINUS).color(ctrl_col).size(16.0)).frame(false)).clicked() {
                            ctx.send_viewport_cmd(egui::ViewportCommand::Minimized(true));
                        }
                        
                        ui.add_space(20.0);
                        // Divider
                        let (rect, _) = ui.allocate_exact_size(Vec2::new(1.0, 16.0), egui::Sense::hover());
                        ui.painter().rect_filled(rect, 0.0, BORDER_COLOR);
                        
                        ui.add_space(20.0);
                        // Action Icons
                        if ui.add(egui::Button::new(RichText::new(egui_phosphor::regular::GEAR).color(TEXT_DIM).size(20.0)).frame(false)).clicked() {
                            self.current_screen = AppScreen::Settings;
                        }
                        ui.add_space(20.0);
                        if ui.add(egui::Button::new(RichText::new(egui_phosphor::regular::TERMINAL_WINDOW).color(TEXT_DIM).size(20.0)).frame(false)).clicked() {
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
                        );

                    });
                });
            });

        if top_response.response.interact(egui::Sense::drag()).dragged() {
            ctx.send_viewport_cmd(egui::ViewportCommand::StartDrag);
        }
    }

"#;

    let pre = &content[..start_idx];
    let post = &content[end_idx..];
    let final_c = format!("{}{}{}", pre, new_bars, post);
    fs::write("src/app.rs", final_c).unwrap();
    println!("Final structure applied!");
}
