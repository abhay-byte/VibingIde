import os
import re

def main():
    with open('src/app.rs', 'r', encoding='utf-8') as f:
        content = f.read()

    # 1. Insert AppScreen enum
    if "enum AppScreen" not in content:
        layout_mode_text = "#[derive(Debug, Clone, Copy, PartialEq, Eq)]\nenum LayoutMode {"
        app_screen_text = """#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AppScreen {
    Editor,
    Agents,
    Settings,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum LayoutMode {"""
        content = content.replace(layout_mode_text, app_screen_text)

    # 2. Update the update method UI rendering
    old_ui_block = """        // 2. Render UI
        self.render_toolbar(ctx);
        self.render_navigation_panels(ctx);
        self.render_panel_switcher(ctx);
        self.render_panels(ctx);"""
    
    new_ui_block = """        // 2. Render Shell & Workspace
        self.render_top_app_bar(ctx);
        
        if self.layout_mode == LayoutMode::Portrait {
             self.render_mobile_bottom_nav(ctx);
        } else {
             self.render_side_nav_bar(ctx);
             self.render_footer(ctx);
        }

        egui::CentralPanel::default()
            .frame(egui::Frame::none().fill(BG_DARK))
            .show(ctx, |ui| {
                 match self.current_screen {
                     AppScreen::Editor => self.render_screen_editor(ui, ctx),
                     AppScreen::Agents => self.render_screen_agents(ui, ctx),
                     AppScreen::Settings => self.render_screen_settings(ui, ctx),
                 }
            });"""
            
    content = content.replace(old_ui_block, new_ui_block)

    with open('src/app.rs', 'w', encoding='utf-8') as f:
        f.write(content)
    
    print("Python rewrite applied!")

if __name__ == '__main__':
    main()
