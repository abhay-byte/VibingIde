use std::fs;

fn main() {
    let mut content = fs::read_to_string("src/app.rs").unwrap();
    
    let old_block = r#"    fn render_screen_editor(&mut self, ui: &mut Ui, ctx: &egui::Context) {
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
    }"#.replace("\r\n", "\n");
    
    let new_block = r#"    fn render_screen_editor(&mut self, ui: &mut Ui, ctx: &egui::Context) {
        self.render_wide_sidebar(ctx);
        egui::CentralPanel::default().frame(egui::Frame::none().fill(BG_DARK)).show_inside(ui, |ui| {
            let available = ui.available_width();
            let editor_width = available * 0.7;
            egui::SidePanel::left("code_editor_panel")
                .exact_width(editor_width)
                .frame(egui::Frame::none().fill(BG_DARK).stroke(egui::Stroke::new(1.0, BG_SIDEBAR)))
                .show_inside(ui, |ui| {
                    ui.horizontal(|ui| {
                        let mut r = ui.allocate_exact_size(Vec2::new(100.0, 32.0), egui::Sense::hover()).0;
                        ui.painter().rect_filled(r, 0.0, BG_PANEL);
                        ui.painter().text(r.center(), egui::Align2::CENTER_CENTER, "main.rs", FontId::proportional(12.0), ACCENT);
                        
                        let mut r2 = ui.allocate_exact_size(Vec2::new(100.0, 32.0), egui::Sense::hover()).0;
                        ui.painter().text(r2.center(), egui::Align2::CENTER_CENTER, "lib.rs", FontId::proportional(12.0), TEXT_DIM);
                    });
                    ui.separator();
                    egui::ScrollArea::vertical().show(ui, |ui| {
                        ui.horizontal(|ui| {
                            ui.vertical(|ui| {
                                for i in 1..=30 {
                                    ui.label(RichText::new(format!(" {:2} ", i)).color(TEXT_DIM).size(12.0));
                                }
                            });
                            ui.vertical(|ui| {
                                let code = "use std::collections::HashMap;\n\n#[derive(Debug)]\npub struct VibingEngine {\n    agents: HashMap<String, AgentState>,\n}\n\nimpl VibingEngine {\n    pub fn new() -> Self {\n        VibingEngine { agents: HashMap::new() }\n    }\n}";
                                ui.label(RichText::new(code).color(TEXT_PRIMARY).size(13.0).family(egui::FontFamily::Monospace));
                            });
                        });
                    });
                });
            
            egui::CentralPanel::default().frame(egui::Frame::none().fill(BG_PANEL)).show_inside(ui, |ui| {
                if self.panel_mgr.panels().is_empty() {
                    self.render_empty_state(ui);
                } else {
                    self.render_panels_portrait(ui, ctx);
                }
            });
        });
    }

    fn render_screen_agents(&mut self, ui: &mut Ui, ctx: &egui::Context) {
        let available = ui.available_width();
        egui::SidePanel::left("agents_left_col")
            .exact_width(available * 0.3)
            .frame(egui::Frame::none().fill(BG_SIDEBAR).stroke(egui::Stroke::new(1.0, BORDER_COLOR)))
            .show_inside(ui, |ui| {
                ui.add_space(8.0);
                ui.heading(RichText::new("  ACTIVE AGENTS").color(ACCENT).size(12.0));
                ui.add_space(8.0);
                for (name, status, cpu) in [("GPT-4o_Debugger", "Running", "12.4%"), ("Claude-3_Refactor", "Idle", "0.1%"), ("Llama-3_Tester", "Reviewing", "45.8%")] {
                    egui::Frame::none().fill(BG_INPUT).inner_margin(8.0).show(ui, |ui| {
                        ui.horizontal(|ui| {
                            ui.label(RichText::new(name).color(TEXT_PRIMARY).strong());
                            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                ui.label(RichText::new(status).color(if status == "Running" { ACCENT } else { TEXT_DIM }).size(10.0));
                            });
                        });
                        ui.horizontal(|ui| {
                            ui.label(RichText::new(format!("CPU: {}", cpu)).color(TEXT_DIM).size(10.0));
                            ui.label(RichText::new("MEM: 1.2GB").color(TEXT_DIM).size(10.0));
                        });
                        ui.horizontal(|ui| {
                            let _ = ui.button(RichText::new("Stop").color(ACCENT_RED).size(10.0));
                            let _ = ui.button(RichText::new(if status == "Running" { "Restart" } else { "Wake" }).color(ACCENT).size(10.0));
                        });
                    });
                    ui.add_space(8.0);
                }
            });

        egui::SidePanel::right("agents_right_col")
            .exact_width(available * 0.25)
            .frame(egui::Frame::none().fill(BG_SIDEBAR).stroke(egui::Stroke::new(1.0, BORDER_COLOR)))
            .show_inside(ui, |ui| {
                ui.add_space(8.0);
                ui.heading(RichText::new("  SYSTEM TELEMETRY").color(TEXT_DIM).size(12.0));
                ui.add_space(8.0);
                ui.label(RichText::new("CPU LOAD: 58%").color(ACCENT));
                ui.add_space(4.0);
                let (rect, _) = ui.allocate_exact_size(Vec2::new(ui.available_width(), 40.0), egui::Sense::hover());
                ui.painter().rect_filled(rect, 0.0, Color32::from_rgba_premultiplied(0, 255, 156, 10));
                ui.painter().line_segment([rect.left_bottom(), rect.right_top() + Vec2::new(0.0, 10.0)], egui::Stroke::new(1.0, ACCENT));

                ui.add_space(16.0);
                ui.label(RichText::new("MEM UTIL: 4.2GB").color(Color32::from_rgb(183, 234, 255)));
                let (rect2, _) = ui.allocate_exact_size(Vec2::new(ui.available_width(), 40.0), egui::Sense::hover());
                ui.painter().rect_filled(rect2, 0.0, Color32::from_rgba_premultiplied(183, 234, 255, 10));
                
                ui.add_space(16.0);
                ui.heading(RichText::new("  OBSERVERS").color(TEXT_DIM).size(12.0));
                ui.label(RichText::new("src/lib/auth.ts  [+12]").color(TEXT_PRIMARY).size(11.0));
                ui.label(RichText::new("package-lock.json [-240]").color(ACCENT_RED).size(11.0));
            });

        egui::CentralPanel::default().frame(egui::Frame::none().fill(BG_PANEL)).show_inside(ui, |ui| {
            if self.panel_mgr.panels().is_empty() {
                self.render_empty_state(ui);
            } else {
                self.render_panels_portrait(ui, ctx);
            }
        });
    }

    fn render_screen_settings(&mut self, ui: &mut Ui, ctx: &egui::Context) {
        egui::SidePanel::left("settings_nav")
            .exact_width(200.0)
            .frame(egui::Frame::none().fill(BG_SIDEBAR).stroke(egui::Stroke::new(1.0, BORDER_COLOR)))
            .show_inside(ui, |ui| {
                ui.add_space(16.0);
                ui.heading(RichText::new("  CONFIGURATION").color(TEXT_DIM).size(10.0).strong());
                ui.add_space(8.0);
                for (i, item) in ["Editor", "Agents", "Keyboard", "Plugins", "Advanced"].iter().enumerate() {
                    let color = if i == 0 { ACCENT } else { TEXT_PRIMARY };
                    let bg = if i == 0 { BG_PANEL } else { Color32::TRANSPARENT };
                    let (rect, _) = ui.allocate_exact_size(Vec2::new(ui.available_width(), 32.0), egui::Sense::hover());
                    ui.painter().rect_filled(rect, 0.0, bg);
                    if i == 0 {
                        ui.painter().rect_filled(egui::Rect::from_min_size(rect.min, Vec2::new(2.0, rect.height())), 0.0, ACCENT);
                    }
                    ui.painter().text(rect.left_center() + Vec2::new(16.0, 0.0), egui::Align2::LEFT_CENTER, *item, FontId::proportional(14.0), color);
                }
            });

        egui::CentralPanel::default().frame(egui::Frame::none().fill(BG_DARK)).show_inside(ui, |ui| {
            egui::ScrollArea::vertical().show(ui, |ui| {
                ui.add_space(16.0);
                ui.heading(RichText::new("Editor Configuration").color(TEXT_PRIMARY).size(18.0).strong());
                ui.add_space(16.0);
                let toml = "[core]\ntheme = \"kinetic-ink-dark\"\nfont_family = \"JetBrains Mono\"\n\n[agents]\nauto_spawn = true\nmax_concurrent = 4";
                let mut t = toml.to_string();
                ui.add(egui::TextEdit::multiline(&mut t)
                    .font(egui::TextStyle::Monospace)
                    .text_color(Color32::from_rgb(183, 234, 255))
                    .desired_width(f32::INFINITY)
                    .desired_rows(10)
                    .frame(true));
                
                ui.add_space(16.0);
                ui.separator();
                ui.add_space(8.0);
                ui.heading(RichText::new("debug.log").color(TEXT_DIM).size(12.0));
                ui.label(RichText::new("[INFO] Loaded config...").color(TEXT_DIM).size(11.0));
            });
        });
    }"#.replace("\r\n", "\n");
    
    let content_normalized = content.replace("\r\n", "\n");
    let result = content_normalized.replace(&old_block, &new_block);
    fs::write("src/app.rs", result).unwrap();
    println!("Mockups replaced with inner components!");
}
