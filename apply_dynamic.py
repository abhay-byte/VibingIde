import os

app_file = "src/app.rs"
with open(app_file, "r", encoding="utf-8") as f:
    content = f.read()

# We need to replace `render_top_app_bar` and `render_side_nav_bar`.
# Let's extract everything from `fn render_side_nav_bar(&mut self` down to `fn render_mobile_bottom_nav`

start_idx = content.find("fn render_side_nav_bar(&mut self, ctx: &egui::Context) {")
end_idx = content.find("fn render_mobile_bottom_nav(&mut self, ctx: &egui::Context) {")

if start_idx == -1 or end_idx == -1:
    print("Could not find targets!")
    exit(1)

new_code = """fn render_side_nav_bar(&mut self, ctx: &egui::Context) {
        egui::SidePanel::left("ultra_slim_rail")
            .exact_width(64.0)
            .frame(egui::Frame::none()
                .fill(BG_PANEL)
                .stroke(egui::Stroke::new(1.0, BG_SIDEBAR)))
            .show(ctx, |ui| {
                ui.vertical_centered(|ui| {
                    ui.add_space(20.0);
                    
                    if self.current_screen == AppScreen::Agents {
                        ui.label(RichText::new("VIBE").color(ACCENT).size(11.0).strong());
                        ui.add_space(2.0);
                        ui.label(RichText::new("V1.0.4").color(TEXT_DIM).size(9.0));
                        ui.add_space(42.0);
                    } else {
                        ui.label(RichText::new("V").color(ACCENT).size(20.0).strong());
                        ui.add_space(42.0);
                    }
                    
                    if ui.add(egui::Button::new(RichText::new(egui_phosphor::regular::FOLDER).size(22.0).color(if self.current_screen == AppScreen::Editor { ACCENT } else { TEXT_DIM })).frame(false)).clicked() {
                        self.current_screen = AppScreen::Editor;
                    }
                    ui.add_space(24.0);
                    if ui.add(egui::Button::new(RichText::new(egui_phosphor::regular::ROBOT).size(22.0).color(if self.current_screen == AppScreen::Agents { ACCENT } else { TEXT_DIM })).frame(false)).clicked() {
                        self.current_screen = AppScreen::Agents;
                    }
                    ui.add_space(24.0);
                    if ui.add(egui::Button::new(RichText::new(egui_phosphor::regular::MAGNIFYING_GLASS).size(20.0).color(TEXT_DIM)).frame(false)).clicked() {}
                    ui.add_space(24.0);
                    if self.current_screen == AppScreen::Settings {
                        if ui.add(egui::Button::new(RichText::new(egui_phosphor::regular::GEAR).size(22.0).color(ACCENT)).frame(false)).clicked() {
                            self.current_screen = AppScreen::Settings;
                        }
                        // Draw green left border indicator for settings
                        let rect = ui.min_rect();
                        ui.painter().rect_filled(
                            egui::Rect::from_min_max(
                                egui::pos2(0.0, rect.top() - 10.0),
                                egui::pos2(3.0, rect.bottom() + 10.0)
                            ),
                            0.0,
                            ACCENT
                        );
                    } else {
                        if ui.add(egui::Button::new(RichText::new(egui_phosphor::regular::GEAR).size(22.0).color(TEXT_DIM)).frame(false)).clicked() {
                            self.current_screen = AppScreen::Settings;
                        }
                    }
                    
                    ui.with_layout(egui::Layout::bottom_up(egui::Align::Center), |ui| {
                        ui.add_space(24.0);
                        if ui.add(egui::Button::new(RichText::new(egui_phosphor::regular::USER_CIRCLE).size(22.0).color(TEXT_DIM)).frame(false)).clicked() {}
                        ui.add_space(24.0);
                    });
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
                    
                    if self.current_screen == AppScreen::Agents {
                        ui.label(RichText::new("VIBINGIDE").color(ACCENT).size(18.0).strong());
                        ui.add_space(32.0);
                        
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
                        
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            ui.add_space(16.0);
                            let ctrl_col = Color32::from_rgb(0, 150, 100);
                            if ui.add(egui::Button::new(RichText::new(egui_phosphor::regular::X).color(ctrl_col).size(16.0)).frame(false)).clicked() { ctx.send_viewport_cmd(egui::ViewportCommand::Close); }
                            ui.add_space(8.0);
                            if ui.add(egui::Button::new(RichText::new(egui_phosphor::regular::SQUARE).color(ctrl_col).size(16.0)).frame(false)).clicked() {
                                let is_max = ctx.input(|i| i.viewport().maximized.unwrap_or(false));
                                ctx.send_viewport_cmd(egui::ViewportCommand::Maximized(!is_max));
                            }
                            ui.add_space(8.0);
                            if ui.add(egui::Button::new(RichText::new(egui_phosphor::regular::MINUS).color(ctrl_col).size(16.0)).frame(false)).clicked() { ctx.send_viewport_cmd(egui::ViewportCommand::Minimized(true)); }
                            
                            ui.add_space(20.0);
                            let (rect, _) = ui.allocate_exact_size(Vec2::new(1.0, 16.0), egui::Sense::hover());
                            ui.painter().rect_filled(rect, 0.0, BORDER_COLOR);
                            ui.add_space(20.0);
                            
                            if ui.add(egui::Button::new(RichText::new(egui_phosphor::regular::GEAR).color(TEXT_DIM).size(20.0)).frame(false)).clicked() { self.current_screen = AppScreen::Settings; }
                            ui.add_space(20.0);
                            if ui.add(egui::Button::new(RichText::new(egui_phosphor::regular::TERMINAL_WINDOW).color(TEXT_DIM).size(20.0)).frame(false)).clicked() { self.current_screen = AppScreen::Editor; }
                            
                            ui.add_space(24.0);
                            let (rect, _) = ui.allocate_exact_size(Vec2::new(260.0, 32.0), egui::Sense::hover());
                            ui.painter().rect_filled(rect, 4.0, BG_INPUT);
                            ui.painter().text(rect.left_center() + Vec2::new(12.0, 0.0), egui::Align2::LEFT_CENTER, format!("{} Search commands...", egui_phosphor::regular::MAGNIFYING_GLASS), FontId::proportional(13.0), TEXT_DIM);
                        });
                        
                    } else if self.current_screen == AppScreen::Editor {
                        ui.label(RichText::new("VibingIDE").color(ACCENT).size(16.0));
                        ui.add_space(32.0);
                        
                        let (rect, _) = ui.allocate_exact_size(Vec2::new(300.0, 32.0), egui::Sense::hover());
                        ui.painter().rect_filled(rect, 4.0, BG_INPUT);
                        ui.painter().text(rect.left_center() + Vec2::new(12.0, 0.0), egui::Align2::LEFT_CENTER, format!("{} Search components or files...", egui_phosphor::regular::MAGNIFYING_GLASS), FontId::proportional(12.0), TEXT_DIM);

                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            ui.add_space(16.0);
                            let ctrl_col = TEXT_DIM;
                            if ui.add(egui::Button::new(RichText::new(egui_phosphor::regular::X).color(ctrl_col).size(16.0)).frame(false)).clicked() { ctx.send_viewport_cmd(egui::ViewportCommand::Close); }
                            ui.add_space(8.0);
                            if ui.add(egui::Button::new(RichText::new(egui_phosphor::regular::SQUARE).color(ctrl_col).size(16.0)).frame(false)).clicked() {
                                let is_max = ctx.input(|i| i.viewport().maximized.unwrap_or(false));
                                ctx.send_viewport_cmd(egui::ViewportCommand::Maximized(!is_max));
                            }
                            ui.add_space(8.0);
                            if ui.add(egui::Button::new(RichText::new(egui_phosphor::regular::MINUS).color(ctrl_col).size(16.0)).frame(false)).clicked() { ctx.send_viewport_cmd(egui::ViewportCommand::Minimized(true)); }
                            
                            ui.add_space(20.0);
                            let (rect, _) = ui.allocate_exact_size(Vec2::new(1.0, 16.0), egui::Sense::hover());
                            ui.painter().rect_filled(rect, 0.0, BORDER_COLOR);
                            ui.add_space(20.0);
                            
                            if ui.add(egui::Button::new(RichText::new(egui_phosphor::regular::LIST).color(TEXT_DIM).size(20.0)).frame(false)).clicked() {}
                            ui.add_space(20.0);
                            if ui.add(egui::Button::new(RichText::new(egui_phosphor::regular::LIGHTNING).color(TEXT_DIM).size(20.0)).frame(false)).clicked() {}
                            ui.add_space(20.0);
                            if ui.add(egui::Button::new(RichText::new(egui_phosphor::regular::TERMINAL_WINDOW).color(TEXT_DIM).size(20.0)).frame(false)).clicked() { self.current_screen = AppScreen::Agents; }
                        });
                    } else if self.current_screen == AppScreen::Settings {
                        ui.label(RichText::new("VibingIDE").color(ACCENT).size(16.0));
                        ui.add_space(16.0);
                        ui.label(RichText::new("/").color(TEXT_DIM).size(16.0));
                        ui.add_space(16.0);
                        ui.label(RichText::new("Project Settings").color(TEXT_PRIMARY).size(16.0).strong());
                        
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            ui.add_space(16.0);
                            let ctrl_col = TEXT_DIM;
                            if ui.add(egui::Button::new(RichText::new(egui_phosphor::regular::X).color(ctrl_col).size(16.0)).frame(false)).clicked() { ctx.send_viewport_cmd(egui::ViewportCommand::Close); }
                            ui.add_space(8.0);
                            if ui.add(egui::Button::new(RichText::new(egui_phosphor::regular::SQUARE).color(ctrl_col).size(16.0)).frame(false)).clicked() {
                                let is_max = ctx.input(|i| i.viewport().maximized.unwrap_or(false));
                                ctx.send_viewport_cmd(egui::ViewportCommand::Maximized(!is_max));
                            }
                            ui.add_space(8.0);
                            if ui.add(egui::Button::new(RichText::new(egui_phosphor::regular::MINUS).color(ctrl_col).size(16.0)).frame(false)).clicked() { ctx.send_viewport_cmd(egui::ViewportCommand::Minimized(true)); }
                            
                            ui.add_space(20.0);
                            let (rect, _) = ui.allocate_exact_size(Vec2::new(1.0, 16.0), egui::Sense::hover());
                            ui.painter().rect_filled(rect, 0.0, BORDER_COLOR);
                            ui.add_space(20.0);
                            
                            if ui.add(egui::Button::new(RichText::new(egui_phosphor::regular::DOTS_THREE_VERTICAL).color(TEXT_DIM).size(20.0)).frame(false)).clicked() {}
                            ui.add_space(20.0);
                            if ui.add(egui::Button::new(RichText::new(egui_phosphor::regular::LIGHTNING).color(TEXT_DIM).size(20.0)).frame(false)).clicked() {}
                            ui.add_space(20.0);
                            if ui.add(egui::Button::new(RichText::new(egui_phosphor::regular::TERMINAL_WINDOW).color(TEXT_DIM).size(20.0)).frame(false)).clicked() { self.current_screen = AppScreen::Agents; }
                            
                            ui.add_space(24.0);
                            let (rect, _) = ui.allocate_exact_size(Vec2::new(260.0, 32.0), egui::Sense::hover());
                            ui.painter().rect_filled(rect, 4.0, BG_INPUT);
                            ui.painter().text(rect.left_center() + Vec2::new(12.0, 0.0), egui::Align2::LEFT_CENTER, format!("{} Search parameters...", egui_phosphor::regular::MAGNIFYING_GLASS), FontId::proportional(13.0), TEXT_DIM);
                        });
                    }
                });
            });

        if top_response.response.interact(egui::Sense::drag()).dragged() {
            ctx.send_viewport_cmd(egui::ViewportCommand::StartDrag);
        }
    }
    fn render_mobile_bottom_nav"""

pre = content[:start_idx]
post = content[end_idx + len("fn render_mobile_bottom_nav"):]
new_content = pre + new_code + post

with open(app_file, "w", encoding="utf-8") as f:
    f.write(new_content)
print("Updated successfully!")
