use std::fs;

fn main() {
    let mut content = fs::read_to_string("src/app.rs").unwrap().replace("\r\n", "\n");

    // 1. Add Infrastructure to AppScreen
    let app_screen_target = "pub enum AppScreen {\n    Editor,\n    Agents,\n    Settings,\n}";
    let app_screen_new = "pub enum AppScreen {\n    Editor,\n    Agents,\n    Settings,\n    Infrastructure,\n}";
    if !content.contains("Infrastructure,") {
        content = content.replace(app_screen_target, app_screen_new);
    }
    
    // 2. Remove render_side_nav_bar from update loop
    let old_update_layout = "        if self.layout_mode == LayoutMode::Portrait {\n             self.render_mobile_bottom_nav(ctx);\n        } else {\n             self.render_side_nav_bar(ctx);\n             self.render_footer(ctx);\n        }";
    let new_update_layout = "        if self.layout_mode == LayoutMode::Portrait {\n             self.render_mobile_bottom_nav(ctx);\n        } else {\n             self.render_footer(ctx);\n        }";
    content = content.replace(old_update_layout, new_update_layout);

    // 3. Add Infrastructure map in update central panel
    let app_match_target = "match self.current_screen {\n                     AppScreen::Editor => self.render_screen_editor(ui, ctx),\n                     AppScreen::Agents => self.render_screen_agents(ui, ctx),\n                     AppScreen::Settings => self.render_screen_settings(ui, ctx),\n                 }";
    let app_match_new = "match self.current_screen {\n                     AppScreen::Editor => self.render_screen_editor(ui, ctx),\n                     AppScreen::Agents => self.render_screen_agents(ui, ctx),\n                     AppScreen::Settings => self.render_screen_settings(ui, ctx),\n                     AppScreen::Infrastructure => { ui.label(RichText::new(\"Infrastructure Dashboard Coming Soon\").color(TEXT_DIM)); }\n                 }";
    if !content.contains("AppScreen::Infrastructure =>") {
        content = content.replace(app_match_target, app_match_new);
    }

    // 4. Overwrite render_top_app_bar completely.
    // The existing method goes from `fn render_top_app_bar` down to `}` before `fn render_side_nav_bar`.
    // I will use regex or string split to replace it.
    let start_idx = content.find("fn render_top_app_bar(&mut self, ctx: &egui::Context) {").unwrap();
    let end_idx = content.find("    fn render_side_nav_bar(&mut self, ctx: &egui::Context) {").unwrap();
    
    let new_top_bar = r#"fn render_top_app_bar(&mut self, ctx: &egui::Context) {
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
                        
                        let response = ui.add_sized([0.0, 48.0], egui::Button::new(text).frame(false));
                        if response.clicked() {
                            self.current_screen = screen;
                        }
                        
                        if active {
                            let rect = response.rect;
                            ui.painter().rect_filled(
                                egui::Rect::from_min_max(
                                    egui::pos2(rect.left(), rect.bottom() + 4.0),
                                    egui::pos2(rect.right(), rect.bottom() + 6.0)
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
                        if ui.add(egui::Button::new(RichText::new(" X ").color(ctrl_col).size(14.0)).frame(false)).clicked() {
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
                        }
                        
                        ui.add_space(20.0);
                        // Divider
                        let (rect, _) = ui.allocate_exact_size(Vec2::new(1.0, 16.0), egui::Sense::hover());
                        ui.painter().rect_filled(rect, 0.0, BORDER_COLOR);
                        
                        ui.add_space(20.0);
                        // Action Icons
                        if ui.add(egui::Button::new(RichText::new("⚙").color(TEXT_PRIMARY).size(18.0)).frame(false)).clicked() {
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
    let final_c = format!("{}{}{}", pre, new_top_bar, post);
    fs::write("src/app.rs", final_c).unwrap();
    println!("TopAppBar Replaced with exact image match!");
}
