use std::fs;

fn main() {
    let mut content = fs::read_to_string("src/app.rs").unwrap().replace("\r\n", "\n");

    // 1. Add sysinfo import
    let import_target = "use std::sync::Arc;";
    let import_new = "use std::sync::Arc;\nuse sysinfo::System;\nuse sysinfo::SystemExt;";
    
    // Quick workaround for sysinfo trait import paths across versions:
    // sysinfo v0.36 uses different traits depending on OS, but basically `System` methods are inherent now.
    // So just `use sysinfo::System;` or `use sysinfo::SystemExt;` if it's < 0.30. It's v0.36.1, so no trait needed!
    let import_new = "use std::sync::Arc;\nuse sysinfo::System;";
    if !content.contains("use sysinfo::System;") {
        content = content.replace(import_target, import_new);
    }

    // 2. Add properties to VibingApp
    let struct_target = "current_screen:       AppScreen,\n}";
    let struct_new = "current_screen:       AppScreen,\n    pub active_file_path: Option<std::path::PathBuf>,\n    pub active_file_content: String,\n    pub settings_toml_buffer: String,\n    pub sys: sysinfo::System,\n}";
    if !content.contains("pub active_file_path") {
        content = content.replace(struct_target, struct_new);
    }

    // 3. Initialize properties in new()
    let init_target = "current_screen: AppScreen::Editor,\n        }";
    let init_new = "current_screen: AppScreen::Editor,\n            active_file_path: None,\n            active_file_content: String::new(),\n            settings_toml_buffer: toml::to_string_pretty(&config).unwrap_or_default(),\n            sys: sysinfo::System::new_all(),\n        }";
    if !content.contains("active_file_path: None,") {
        content = content.replace(init_target, init_new);
    }

    // 4. Update update() to refresh sysinfo
    let update_target = "self.drain_pty_events();";
    let update_new = "self.drain_pty_events();\n        self.sys.refresh_cpu_usage();\n        self.sys.refresh_memory();";
    if !content.contains("self.sys.refresh_cpu_usage()") {
        content = content.replace(update_target, update_new);
    }

    // 5. Update render_nodes signature
    let render_nodes_target = "fn render_nodes(ui: &mut Ui, nodes: &[crate::engine::project::FileNode], depth: usize) {";
    let render_nodes_new = "fn render_nodes(ui: &mut Ui, nodes: &[crate::engine::project::FileNode], depth: usize) -> Option<std::path::PathBuf> {
    let mut clicked = None;";
    if !content.contains("-> Option<std::path::PathBuf>") {
        content = content.replace(render_nodes_target, render_nodes_new);
        
        let rn_body_target = "if response.hovered() {
                response.on_hover_cursor(egui::CursorIcon::PointingHand);
            }
        });
        if node.is_dir() && !node.children.is_empty() {
            render_nodes(ui, &node.children, depth + 1);
        }";
        let rn_body_new = "if response.clicked() && !node.is_dir() {
                clicked = Some(node.path.clone());
            }
            if response.hovered() {
                response.on_hover_cursor(egui::CursorIcon::PointingHand);
            }
        });
        if node.is_dir() && !node.children.is_empty() {
            if let Some(p) = render_nodes(ui, &node.children, depth + 1) {
                clicked = Some(p);
            }
        }";
        content = content.replace(rn_body_target, rn_body_new);
        
        let rn_end_target = "    }\n}\n\n// Trait extension helper";
        let rn_end_new = "    }\n    clicked\n}\n\n// Trait extension helper";
        content = content.replace(rn_end_target, rn_end_new);
    }

    // 6. Update render_file_tree signature
    let file_tree_target = "fn render_file_tree(&self, ui: &mut Ui) {";
    let file_tree_new = "fn render_file_tree(&mut self, ui: &mut Ui) {";
    content = content.replace(file_tree_target, file_tree_new);
    
    let render_file_tree_body_target = "render_nodes(ui, &self.project.file_tree, 0);";
    let render_file_tree_body_new = "if let Some(p) = render_nodes(ui, &self.project.file_tree, 0) {
                    self.active_file_path = Some(p.clone());
                    if let Ok(buf) = std::fs::read_to_string(&p) {
                        self.active_file_content = buf;
                    }
                }";
    if !content.contains("self.active_file_path = Some(p") {
        content = content.replace(render_file_tree_body_target, render_file_tree_body_new);
    }
    
    // 7. Update Toolbar buttons for Close / Min / Max
    let old_buttons = r#"if ui.add(egui::Button::new(RichText::new(" ✕ ").color(ACCENT_RED).size(14.0)).frame(false)).clicked() {
                            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                        }"#;
    let new_buttons = r#"if ui.add(egui::Button::new(RichText::new(" ✕ ").color(ACCENT_RED).size(14.0)).frame(false)).clicked() {
                            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                        }
                        if ui.add(egui::Button::new(RichText::new(" 🗖 ").color(TEXT_PRIMARY).size(14.0)).frame(false)).clicked() {
                            let is_max = ctx.input(|i| i.viewport().maximized.unwrap_or(false));
                            ctx.send_viewport_cmd(egui::ViewportCommand::Maximized(!is_max));
                        }
                        if ui.add(egui::Button::new(RichText::new(" 🗕 ").color(TEXT_PRIMARY).size(14.0)).frame(false)).clicked() {
                            ctx.send_viewport_cmd(egui::ViewportCommand::Minimized(true));
                        }"#;
    
    let top_bar_controls_old = r#"if ui.add(egui::Button::new(RichText::new(" ☰ ").color(TEXT_PRIMARY).size(18.0)).frame(false)).clicked() {}
                        if ui.add(egui::Button::new(RichText::new(" ⚡ ").color(TEXT_PRIMARY).size(18.0)).frame(false)).clicked() {}
                        if ui.add(egui::Button::new(RichText::new(" >_ ").color(TEXT_PRIMARY).size(18.0)).frame(false)).clicked() {}"#;
    
    if !content.contains("ctx.send_viewport_cmd(egui::ViewportCommand::Close);") {
        content = content.replace(top_bar_controls_old, new_buttons);
    }
    
    // Window dragging support in Top Bar
    let top_bar_end_target = "});\n            });\n    }";
    let top_bar_end_new = "});\n            });\n        if response.response.interact(egui::Sense::drag()).dragged() {\n            ctx.send_viewport_cmd(egui::ViewportCommand::StartDrag);\n        }\n    }";
    let old_top_bar_start = "egui::TopBottomPanel::top(\"top_app_bar\")";
    let new_top_bar_start = "let response = egui::TopBottomPanel::top(\"top_app_bar\")";
    if !content.contains("ctx.send_viewport_cmd(egui::ViewportCommand::StartDrag);") {
        content = content.replace(top_bar_end_target, top_bar_end_new);
        content = content.replace(old_top_bar_start, new_top_bar_start);
    }
    
    fs::write("src/app.rs", content).unwrap();
    println!("State injections applied!");
}
