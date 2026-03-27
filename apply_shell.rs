use std::fs;

fn main() {
    let mut content = fs::read_to_string("src/app.rs").unwrap();
    
    let layout_mode_text = "#[derive(Debug, Clone, Copy, PartialEq, Eq)]\nenum LayoutMode {";
    let app_screen_text = "#[derive(Debug, Clone, Copy, PartialEq, Eq)]\npub enum AppScreen {\n    Editor,\n    Agents,\n    Settings,\n}\n\n#[derive(Debug, Clone, Copy, PartialEq, Eq)]\nenum LayoutMode {";
    
    // Support varying line endings by replacing CRLF with LF in strings
    let layout_mode_text = layout_mode_text.replace("\r\n", "\n");
    let mut content_normalized = content.replace("\r\n", "\n");
    
    if !content_normalized.contains("enum AppScreen") {
        content_normalized = content_normalized.replace(&layout_mode_text, &app_screen_text);
    }
    
    let old_ui_block = "        // 2. Render UI\n        self.render_toolbar(ctx);\n        self.render_navigation_panels(ctx);\n        self.render_panel_switcher(ctx);\n        self.render_panels(ctx);";
    
    let new_ui_block = "        // 2. Render Shell & Workspace\n        self.render_top_app_bar(ctx);\n        \n        if self.layout_mode == LayoutMode::Portrait {\n             self.render_mobile_bottom_nav(ctx);\n        } else {\n             self.render_side_nav_bar(ctx);\n             self.render_footer(ctx);\n        }\n\n        egui::CentralPanel::default()\n            .frame(egui::Frame::none().fill(BG_DARK))\n            .show(ctx, |ui| {\n                 match self.current_screen {\n                     AppScreen::Editor => self.render_screen_editor(ui, ctx),\n                     AppScreen::Agents => self.render_screen_agents(ui, ctx),\n                     AppScreen::Settings => self.render_screen_settings(ui, ctx),\n                 }\n            });";
    
    content_normalized = content_normalized.replace(old_ui_block, new_ui_block);
    
    fs::write("src/app.rs", content_normalized).unwrap();
    println!("Rust rewrite applied!");
}
