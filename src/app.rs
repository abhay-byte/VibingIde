//! VibingIDE — egui Application
//!
//! Implements `eframe::App`. Owns all state; communicates with PTY supervisor
//! tasks via a tokio mpsc channel polled synchronously in `update()`.

#![allow(dead_code, unused_variables)]

use std::path::PathBuf;
use std::sync::Arc;

use egui::{Color32, Event, FontId, Key, KeyboardShortcut, Modifiers, RichText, Ui, Vec2};
use serde::{Deserialize, Serialize};
use tokio::sync::mpsc;
use tracing::{debug, warn};

use crate::config::AppConfig;
use crate::engine::{
    panel_manager::{PanelManager, PanelStatus},
    project::Project,
    session_manager::SessionManager,
};
use crate::pty::supervisor::PtyEvent;

// ── Design tokens ──────────────────────────────────────────────────────────────

const BG_DARK:       Color32 = Color32::from_rgb(0x13, 0x13, 0x13);
const BG_PANEL:      Color32 = Color32::from_rgb(0x0e, 0x0e, 0x0e);
const BG_SIDEBAR:    Color32 = Color32::from_rgb(0x1c, 0x1b, 0x1b);
const BG_INPUT:      Color32 = Color32::from_rgb(0x35, 0x35, 0x34);
const BORDER_COLOR:  Color32 = Color32::from_rgba_premultiplied(9, 11, 9, 38);
const ACCENT:        Color32 = Color32::from_rgb(0x56, 0xff, 0xa7);
const ACCENT_GREEN:  Color32 = Color32::from_rgb(0x56, 0xff, 0xa7);
const ACCENT_RED:    Color32 = Color32::from_rgb(0xff, 0xb4, 0xab);
const ACCENT_YELLOW: Color32 = Color32::from_rgb(0xff, 0xdc, 0xbb);
const TEXT_PRIMARY:  Color32 = Color32::from_rgb(0xe5, 0xe2, 0xe1);
const TEXT_DIM:      Color32 = Color32::from_rgb(0xb9, 0xcb, 0xbc);
const TEXT_MONO:     &str    = "JetBrains Mono";
const ZOOM_STEP:     f32     = 0.1;
const MIN_ZOOM:      f32     = 0.7;
const MAX_ZOOM:      f32     = 2.0;
const ZOOM_APPLY_TOLERANCE: f32 = 0.01;
const BASE_WIDTH:    f32     = 1280.0;
const BASE_HEIGHT:   f32     = 800.0;
const MIN_AUTO_SCALE:f32     = 0.85;
const MAX_AUTO_SCALE:f32     = 1.0;
const PORTRAIT_BREAKPOINT: f32 = 900.0;
const PORTRAIT_ASPECT_THRESHOLD: f32 = 1.05;
const PORTRAIT_EXIT_BREAKPOINT: f32 = 940.0;
const PORTRAIT_EXIT_ASPECT_THRESHOLD: f32 = 1.0;
const PORTRAIT_WORKSPACE_MIN_HEIGHT: f32 = 140.0;
const PORTRAIT_WORKSPACE_MAX_HEIGHT: f32 = 280.0;
const PORTRAIT_WORKSPACE_FRACTION: f32 = 0.30;
const PANEL_HEADER_HEIGHT: f32 = 30.0;
const PANEL_INPUT_HEIGHT: f32 = 34.0;
const PERSISTED_UI_KEY: &str = "vibingide_ui_state";

// ── App state ──────────────────────────────────────────────────────────────────

/// Which view is active in the left sidebar.
#[derive(PartialEq, Clone)]
enum SidebarView { Files, History }

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AppScreen {
    Editor,
    Agents,
    Settings,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum LayoutMode {
    Wide,
    Portrait,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct PersistedUiState {
    manual_zoom_factor: f32,
    show_portrait_workspace: bool,
}

impl PersistedUiState {
    fn from_config(config: &AppConfig) -> Self {
        Self {
            manual_zoom_factor: config.ui.zoom_factor,
            show_portrait_workspace: true,
        }
    }
}

pub struct VibingApp {
    // Core subsystems
    project:     Project,
    config:      AppConfig,
    session_mgr: SessionManager,
    panel_mgr:   PanelManager,

    // PTY event receiver (polled each frame)
    pty_rx: mpsc::UnboundedReceiver<PtyEvent>,

    // Keep runtime alive
    _rt: Arc<tokio::runtime::Runtime>,

    // UI state
    sidebar_view:         SidebarView,
    show_help:            bool,
    show_new_panel_dialog: bool,
    new_panel_cmd:        String,
    cmd_error:            Option<String>,
    auto_scale:           bool,
    manual_zoom_factor:   f32,
    applied_zoom_factor:  f32,
    show_portrait_workspace: bool,
    layout_mode:          LayoutMode,
    current_screen:       AppScreen,
}

fn split_cmdline(s: &str) -> Option<(String, Vec<String>)> {
    let mut parts = s.split_whitespace().map(|p| p.to_string());
    if let Some(cmd) = parts.next() {
        Some((cmd, parts.collect()))
    } else {
        None
    }
}

impl VibingApp {
    pub fn new(
        cc:          &eframe::CreationContext,
        project_root: PathBuf,
        config:       AppConfig,
        initial_cmd:  Option<String>,
        rt:           Arc<tokio::runtime::Runtime>,
    ) -> Self {
        let persisted_ui = cc
            .storage
            .and_then(|storage| eframe::get_value::<PersistedUiState>(storage, PERSISTED_UI_KEY))
            .unwrap_or_else(|| PersistedUiState::from_config(&config));
        let auto_scale = config.ui.auto_scale;
        let manual_zoom_factor = persisted_ui
            .manual_zoom_factor
            .clamp(MIN_ZOOM, MAX_ZOOM);
        let project     = Project::open(project_root.clone()).expect("open project");
        let session_mgr = SessionManager::load(&project.vibide_dir).unwrap_or_default();

        let (pty_tx, pty_rx) = mpsc::unbounded_channel::<PtyEvent>();
        let mut panel_mgr = PanelManager::new(
            pty_tx,
            config.ui.output_buffer_lines,
            config.security.child_env_allowlist.clone(),
            rt.handle().clone(),
            project_root,
        );

        if let Some(cmd_str) = initial_cmd {
            let session_id = SessionManager::new_session_id();
            if let Some((command, args)) = split_cmdline(&cmd_str) {
                if let Err(e) = panel_mgr.create_panel(command, args, session_id, (220, 50)) {
                    warn!("Initial panel failed: {e}");
                }
            }
        }

        Self {
            project,
            config,
            session_mgr,
            panel_mgr,
            pty_rx,
            _rt: rt,
            sidebar_view:          SidebarView::Files,
            show_help:             false,
            show_new_panel_dialog: false,
            new_panel_cmd:         String::new(),
            cmd_error:             None,
            auto_scale,
            manual_zoom_factor,
            applied_zoom_factor:   manual_zoom_factor,
            show_portrait_workspace: persisted_ui.show_portrait_workspace,
            layout_mode: LayoutMode::Wide,
            current_screen: AppScreen::Editor,
        }
    }

    /// Configure the egui visual style.
    pub fn setup_visuals(ctx: &egui::Context) {
        let mut visuals = egui::Visuals::dark();
        visuals.window_fill         = BG_DARK;
        visuals.panel_fill          = BG_PANEL;
        visuals.faint_bg_color      = BG_SIDEBAR;
        visuals.extreme_bg_color    = BG_INPUT;
        visuals.window_rounding     = egui::Rounding::same(2.0);
        visuals.widgets.noninteractive.bg_fill = BG_PANEL;
        visuals.widgets.inactive.bg_fill       = BG_INPUT;
        visuals.widgets.hovered.bg_fill        = Color32::from_rgb(0x35, 0x35, 0x34);
        visuals.widgets.active.bg_fill         = ACCENT;
        visuals.selection.bg_fill              = Color32::from_rgb(0x00, 0x71, 0x42);
        ctx.set_visuals(visuals);

        let mut fonts = egui::FontDefinitions::default();
        fonts.font_data.insert(
            "JetBrains Mono".to_owned(),
            egui::FontData::from_static(include_bytes!("../assets/JetBrainsMono-Regular.ttf")),
        );
        fonts
            .families
            .entry(egui::FontFamily::Monospace)
            .or_default()
            .insert(0, "JetBrains Mono".to_owned());

        ctx.set_fonts(fonts);
        ctx.options_mut(|opt| opt.zoom_with_keyboard = false);

        ctx.set_style({
            let mut style = (*ctx.style()).clone();
            style.spacing.item_spacing   = Vec2::new(6.0, 4.0);
            style.spacing.button_padding = Vec2::new(12.0, 6.0);
            style
        });
    }

    fn handle_zoom_shortcuts(&mut self, ctx: &egui::Context) {
        let zoom_in = KeyboardShortcut::new(Modifiers::COMMAND, Key::Plus);
        let zoom_in_secondary = KeyboardShortcut::new(Modifiers::COMMAND, Key::Equals);
        let zoom_out = KeyboardShortcut::new(Modifiers::COMMAND, Key::Minus);
        let zoom_reset = KeyboardShortcut::new(Modifiers::COMMAND, Key::Num0);

        if ctx.input_mut(|i| i.consume_shortcut(&zoom_reset)) {
            self.manual_zoom_factor = self.config.ui.zoom_factor;
            return;
        }

        if ctx.input_mut(|i| i.consume_shortcut(&zoom_in))
            || ctx.input_mut(|i| i.consume_shortcut(&zoom_in_secondary))
        {
            self.manual_zoom_factor = (self.manual_zoom_factor + ZOOM_STEP)
                .clamp(MIN_ZOOM, MAX_ZOOM);
            self.manual_zoom_factor = (self.manual_zoom_factor * 10.0).round() / 10.0;
        }

        if ctx.input_mut(|i| i.consume_shortcut(&zoom_out)) {
            self.manual_zoom_factor = (self.manual_zoom_factor - ZOOM_STEP)
                .clamp(MIN_ZOOM, MAX_ZOOM);
            self.manual_zoom_factor = (self.manual_zoom_factor * 10.0).round() / 10.0;
        }
    }

    fn auto_zoom_factor(&self, ctx: &egui::Context) -> f32 {
        if !self.auto_scale {
            return 1.0;
        }

        auto_zoom_factor_for_size(viewport_size_for_layout(ctx))
    }

    fn apply_zoom(&mut self, ctx: &egui::Context) {
        let target_zoom = normalized_zoom_factor(
            (self.manual_zoom_factor * self.auto_zoom_factor(ctx))
                .clamp(MIN_ZOOM, MAX_ZOOM)
        );

        if (ctx.zoom_factor() - target_zoom).abs() >= ZOOM_APPLY_TOLERANCE {
            debug!(
                current_zoom = ctx.zoom_factor(),
                target_zoom,
                manual_zoom = self.manual_zoom_factor,
                "Applying zoom factor"
            );
            ctx.set_zoom_factor(target_zoom);
        }

        self.applied_zoom_factor = target_zoom;
    }

    fn layout_mode(&self) -> LayoutMode {
        self.layout_mode
    }

    fn persisted_ui_state(&self) -> PersistedUiState {
        PersistedUiState {
            manual_zoom_factor: self.manual_zoom_factor,
            show_portrait_workspace: self.show_portrait_workspace,
        }
    }

    fn handle_terminal_input(&mut self, ctx: &egui::Context) {
        if self.show_new_panel_dialog || self.show_help {
            return;
        }

        let Some(panel_id) = self.panel_mgr.focused_id() else {
            return;
        };
        let Some(panel) = self.panel_mgr.panel(panel_id) else {
            return;
        };
        if !matches!(panel.status, PanelStatus::Running { .. }) {
            return;
        }

        let application_cursor = panel.terminal.application_cursor();
        let events = ctx.input(|i| i.events.clone());
        for event in events {
            if let Some(bytes) = terminal_bytes_for_event(&event, application_cursor) {
                let _ = self.panel_mgr.send_bytes(panel_id, &bytes);
            }
        }
    }

    // ── PTY event drain ────────────────────────────────────────────────────────

    fn drain_pty_events(&mut self) {
        loop {
            match self.pty_rx.try_recv() {
                Ok(PtyEvent::Output { panel_id, data }) => {
                    self.panel_mgr.handle_output(panel_id, data);
                }
                Ok(PtyEvent::Exited { panel_id, exit_code }) => {
                    self.panel_mgr.handle_exit(panel_id, exit_code);
                }
                Err(_) => break,
            }
        }
    }

    // ── "New Panel" dialog ─────────────────────────────────────────────────────

    fn show_new_panel_dialog(&mut self, ctx: &egui::Context) {
        let mut open = self.show_new_panel_dialog;
        egui::Window::new("New Agent Panel")
            .open(&mut open)
            .resizable(false)
            .collapsible(false)
            .default_width(460.0)
            .anchor(egui::Align2::CENTER_CENTER, Vec2::ZERO)
            .frame(egui::Frame::window(&ctx.style())
                .fill(BG_PANEL)
                .rounding(12.0)
                .stroke(egui::Stroke::new(1.0, ACCENT)))
            .show(ctx, |ui| {
                ui.add_space(8.0);
                ui.label(RichText::new("Command to launch").color(TEXT_DIM).size(12.0));
                ui.add_space(4.0);

                let response = ui.add(
                    egui::TextEdit::singleline(&mut self.new_panel_cmd)
                        .desired_width(f32::INFINITY)
                        .font(FontId::monospace(14.0))
                        .hint_text("e.g.  claude  /  opencode  /  cmd.exe")
                );
                if response.lost_focus() && ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
                    self.show_new_panel_dialog = false;
                    self.new_panel_cmd.clear();
                    return;
                }
                response.request_focus();

                ui.add_space(12.0);
                ui.horizontal(|ui| {
                    let launch = ui.add(
                        egui::Button::new(
                            RichText::new("  +  Launch  ")
                                .color(Color32::WHITE)
                                .size(14.0)
                                .strong()
                        )
                        .fill(ACCENT)
                        .rounding(8.0)
                        .min_size(Vec2::new(120.0, 36.0))
                    );
                    let cancel = ui.add(
                        egui::Button::new(
                            RichText::new("  Cancel  ").color(TEXT_DIM).size(14.0)
                        )
                        .fill(BG_INPUT)
                        .rounding(8.0)
                        .min_size(Vec2::new(90.0, 36.0))
                    );

                    let submit = launch.clicked()
                        || (ctx.input(|i| i.key_pressed(egui::Key::Enter))
                            && !self.new_panel_cmd.trim().is_empty());

                    if submit {
                        let cmd_str = self.new_panel_cmd.trim().to_string();
                        if let Some((command, args)) = split_cmdline(&cmd_str) {
                            let session_id = SessionManager::new_session_id();
                            match self.panel_mgr.create_panel(command, args, session_id, (220, 50)) {
                                Ok(_) => {
                                    self.show_new_panel_dialog = false;
                                    self.new_panel_cmd.clear();
                                    self.cmd_error = None;
                                }
                                Err(e) => {
                                    self.cmd_error = Some(format!("Failed to start: {e}"));
                                }
                            }
                        }
                    }
                    if cancel.clicked() {
                        self.show_new_panel_dialog = false;
                        self.new_panel_cmd.clear();
                        self.cmd_error = None;
                    }
                });
                if let Some(err) = &self.cmd_error {
                    ui.add_space(8.0);
                    ui.label(RichText::new(err).color(ACCENT_RED).size(12.0));
                }
                ui.add_space(4.0);
            });

        if !open { self.show_new_panel_dialog = false; }
    }

    // ── Help overlay ──────────────────────────────────────────────────────────

    fn show_help_window(&mut self, ctx: &egui::Context) {
        let mut open = self.show_help;
        egui::Window::new("Keyboard Shortcuts")
            .open(&mut open)
            .resizable(false)
            .collapsible(false)
            .default_width(420.0)
            .anchor(egui::Align2::CENTER_CENTER, Vec2::ZERO)
            .frame(egui::Frame::window(&ctx.style())
                .fill(BG_PANEL)
                .rounding(12.0)
                .stroke(egui::Stroke::new(1.0, ACCENT)))
            .show(ctx, |ui| {
                let entries: &[(&str, &str)] = &[
                    ("Ctrl+N",       "New agent panel"),
                    ("Ctrl++ / Ctrl+=", "Scale UI up"),
                    ("Ctrl+-",       "Scale UI down"),
                    ("Ctrl+0",       "Reset manual scale"),
                    ("Ctrl+]",       "Focus next panel"),
                    ("Ctrl+[",       "Focus previous panel"),
                    ("Ctrl+W",       "Close focused panel"),
                    ("Ctrl+C",       "Send SIGINT to focused terminal"),
                    ("Ctrl+Shift+Q", "Quit VibingIDE"),
                    ("Click panel",  "Focus that panel"),
                    ("Click [X]",    "Close that panel"),
                    ("Type directly", "Send keys to focused terminal"),
                ];
                for (key, desc) in entries {
                    ui.horizontal(|ui| {
                        ui.label(RichText::new(*key).color(ACCENT_YELLOW).monospace().size(13.0));
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            ui.label(RichText::new(*desc).color(TEXT_PRIMARY).size(13.0));
                        });
                    });
                    ui.separator();
                }
            });
        if !open { self.show_help = false; }
    }

    // ── Toolbar ───────────────────────────────────────────────────────────────

    fn render_toolbar(&mut self, ctx: &egui::Context) {
        let layout_mode = self.layout_mode();
        let toolbar_height = if layout_mode == LayoutMode::Portrait { 74.0 } else { 42.0 };
        egui::TopBottomPanel::top("toolbar")
            .exact_height(toolbar_height)
            .frame(egui::Frame::none()
                .fill(Color32::from_rgb(12, 12, 20))
                .inner_margin(egui::Margin::symmetric(12.0, 6.0))
                .stroke(egui::Stroke::new(1.0, Color32::from_rgb(40, 40, 65))))
            .show(ctx, |ui| {
                // Drag window to move it
                let rect = ui.max_rect();
                let resp = ui.interact(rect, ui.id().with("title_bar"), egui::Sense::click_and_drag());
                if resp.dragged() {
                    ctx.send_viewport_cmd(egui::ViewportCommand::StartDrag);
                }

                if layout_mode == LayoutMode::Portrait {
                    ui.vertical_centered(|ui| {
                        ui.horizontal_wrapped(|ui| {
                            ui.label(
                                RichText::new("VibingIDE")
                                    .size(16.0)
                                    .strong()
                                    .color(Color32::from_rgb(160, 130, 255))
                            );
                            ui.label(
                                RichText::new(&self.project.name)
                                    .size(12.0)
                                    .color(TEXT_DIM)
                            );
                            ui.label(
                                RichText::new(format!("{:.0}% zoom", self.applied_zoom_factor * 100.0))
                                    .color(TEXT_DIM)
                                    .size(11.0)
                            );
                        });
                        ui.horizontal_wrapped(|ui| {
                            if ui.add(
                                egui::Button::new(
                                    RichText::new(if self.show_portrait_workspace { " Hide Workspace " } else { " Show Workspace " })
                                        .color(Color32::WHITE)
                                        .size(12.0)
                                )
                                .fill(Color32::from_rgb(50, 74, 120))
                                .rounding(6.0)
                            ).clicked() {
                                self.show_portrait_workspace = !self.show_portrait_workspace;
                            }
                            if ui.add(
                                egui::Button::new(
                                    RichText::new(" + New Panel ").color(Color32::WHITE).size(12.0).strong()
                                )
                                .fill(Color32::from_rgb(60, 160, 100))
                                .rounding(6.0)
                            ).clicked() {
                                self.show_new_panel_dialog = true;
                            }
                            let help_fill = if self.show_help {
                                Color32::from_rgb(60, 90, 180)
                            } else {
                                Color32::from_rgb(35, 50, 90)
                            };
                            if ui.add(
                                egui::Button::new(RichText::new(" ? Help ").color(Color32::WHITE).size(12.0))
                                    .fill(help_fill)
                                    .rounding(6.0)
                            ).clicked() {
                                self.show_help = !self.show_help;
                            }
                            self.render_window_controls(ctx, ui);
                        });
                    });
                } else {
                    ui.horizontal_centered(|ui| {
                        ui.label(
                            RichText::new("VibingIDE")
                                .size(16.0)
                                .strong()
                                .color(Color32::from_rgb(160, 130, 255))
                        );
                        ui.label(
                            RichText::new(format!("  {}", self.project.name))
                                .size(13.0)
                                .color(TEXT_DIM)
                        );

                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            self.render_window_controls(ctx, ui);
                            ui.add_space(12.0);

                            ui.label(
                                RichText::new(format!("{:.0}% zoom", self.applied_zoom_factor * 100.0))
                                    .color(TEXT_DIM)
                                    .size(12.0)
                            );
                            ui.add_space(8.0);

                            let help_fill = if self.show_help {
                                Color32::from_rgb(60, 90, 180)
                            } else {
                                Color32::from_rgb(35, 50, 90)
                            };
                            if ui.add(
                                egui::Button::new(RichText::new(" ? Help ").color(Color32::WHITE).size(13.0))
                                    .fill(help_fill)
                                    .rounding(6.0)
                            ).clicked() {
                                self.show_help = !self.show_help;
                            }
                            ui.add_space(6.0);

                            if ui.add(
                                egui::Button::new(
                                    RichText::new(" + New Panel ").color(Color32::WHITE).size(13.0).strong()
                                )
                                .fill(Color32::from_rgb(60, 160, 100))
                                .rounding(6.0)
                            ).clicked() {
                                self.show_new_panel_dialog = true;
                            }
                            ui.add_space(6.0);
                        });
                    });
                }
            });
    }

    fn render_window_controls(&self, ctx: &egui::Context, ui: &mut Ui) {
        if ui.add(
            egui::Button::new(RichText::new(" ✕ ").color(Color32::WHITE).size(13.0))
                .fill(Color32::from_rgb(160, 40, 40))
                .rounding(6.0)
        ).clicked() {
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
        }

        if ui.add(
            egui::Button::new(RichText::new(" □ ").color(TEXT_PRIMARY).size(13.0))
                .fill(Color32::from_rgb(40, 40, 65))
                .rounding(6.0)
        ).clicked() {
            let is_max = ctx.input(|i| i.viewport().maximized.unwrap_or(false));
            ctx.send_viewport_cmd(egui::ViewportCommand::Maximized(!is_max));
        }

        if ui.add(
            egui::Button::new(RichText::new(" _ ").color(TEXT_PRIMARY).size(13.0))
                .fill(Color32::from_rgb(40, 40, 65))
                .rounding(6.0)
        ).clicked() {
            ctx.send_viewport_cmd(egui::ViewportCommand::Minimized(true));
        }
    }

    fn render_navigation_panels(&mut self, ctx: &egui::Context) {
        match self.layout_mode() {
            LayoutMode::Wide => self.render_wide_sidebar(ctx),
            LayoutMode::Portrait => self.render_portrait_workspace(ctx),
        }
    }

    // ── Navigation panels ─────────────────────────────────────────────────────

    fn render_wide_sidebar(&mut self, ctx: &egui::Context) {
        egui::SidePanel::left("sidebar")
            .resizable(true)
            .default_width(240.0)
            .min_width(160.0)
            .max_width(400.0)
            .frame(egui::Frame::none()
                .fill(BG_SIDEBAR)
                .stroke(egui::Stroke::new(1.0, BORDER_COLOR)))
            .show(ctx, |ui| {
                ui.add_space(6.0);

                // Tab bar
                ui.horizontal(|ui| {
                    ui.add_space(8.0);
                    let files_active   = self.sidebar_view == SidebarView::Files;
                    let history_active = self.sidebar_view == SidebarView::History;

                    if ui.add(tab_button("Files", files_active)).clicked() {
                        self.sidebar_view = SidebarView::Files;
                    }
                    ui.add_space(4.0);
                    if ui.add(tab_button("History", history_active)).clicked() {
                        self.sidebar_view = SidebarView::History;
                    }
                });

                ui.add_space(6.0);
                ui.add(egui::Separator::default().horizontal());
                ui.add_space(4.0);

                match self.sidebar_view {
                    SidebarView::Files   => self.render_file_tree(ui),
                    SidebarView::History => self.render_history_list(ui),
                }
            });
    }

    fn render_portrait_workspace(&mut self, ctx: &egui::Context) {
        if !self.show_portrait_workspace {
            return;
        }

        let height = ctx.input(|i| i.screen_rect().height());
        let workspace_height = (height * PORTRAIT_WORKSPACE_FRACTION)
            .clamp(PORTRAIT_WORKSPACE_MIN_HEIGHT, PORTRAIT_WORKSPACE_MAX_HEIGHT);

        egui::TopBottomPanel::top("portrait_workspace")
            .exact_height(workspace_height)
            .frame(egui::Frame::none()
                .fill(BG_SIDEBAR)
                .stroke(egui::Stroke::new(1.0, BORDER_COLOR)))
            .show(ctx, |ui| {
                ui.add_space(6.0);
                ui.horizontal_wrapped(|ui| {
                    let files_active = self.sidebar_view == SidebarView::Files;
                    let history_active = self.sidebar_view == SidebarView::History;

                    if ui.add(tab_button("Files", files_active)).clicked() {
                        self.sidebar_view = SidebarView::Files;
                    }
                    if ui.add(tab_button("History", history_active)).clicked() {
                        self.sidebar_view = SidebarView::History;
                    }
                });
                ui.add_space(6.0);
                ui.separator();
                ui.add_space(4.0);

                match self.sidebar_view {
                    SidebarView::Files => self.render_file_tree(ui),
                    SidebarView::History => self.render_history_list(ui),
                }
            });
    }

    fn render_panel_switcher(&mut self, ctx: &egui::Context) {
        if self.layout_mode() != LayoutMode::Portrait || self.panel_mgr.panels().is_empty() {
            return;
        }

        let panel_tabs: Vec<(u32, String, PanelStatus)> = self
            .panel_mgr
            .panels()
            .iter()
            .map(|panel| (panel.id, panel.label.clone(), panel.status.clone()))
            .collect();

        egui::TopBottomPanel::top("portrait_panel_switcher")
            .exact_height(52.0)
            .frame(egui::Frame::none()
                .fill(Color32::from_rgb(16, 16, 24))
                .stroke(egui::Stroke::new(1.0, BORDER_COLOR)))
            .show(ctx, |ui| {
                egui::ScrollArea::horizontal()
                    .auto_shrink([false, true])
                    .show(ui, |ui| {
                        ui.horizontal(|ui| {
                            for (panel_id, label, status) in &panel_tabs {
                                let focused = self.panel_mgr.focused_id() == Some(*panel_id);
                                let fill = if focused { ACCENT } else { BG_INPUT };
                                let status_color = match status {
                                    PanelStatus::Running { .. } => ACCENT_GREEN,
                                    PanelStatus::Starting => ACCENT_YELLOW,
                                    PanelStatus::Exited { .. } => TEXT_DIM,
                                    PanelStatus::Crashed { .. } => ACCENT_RED,
                                };

                                let label = RichText::new(label)
                                    .color(Color32::WHITE)
                                    .size(12.0)
                                    .strong();

                                if ui.add(
                                    egui::Button::new(label)
                                        .fill(fill)
                                        .stroke(egui::Stroke::new(1.0, status_color))
                                        .rounding(8.0)
                                ).clicked() {
                                    self.panel_mgr.set_focus(*panel_id);
                                }
                            }
                        });
                    });
            });
    }

    fn render_file_tree(&self, ui: &mut Ui) {
        egui::ScrollArea::vertical()
            .auto_shrink([false, false])
            .show(ui, |ui| {
                ui.add_space(2.0);
                render_nodes(ui, &self.project.file_tree, 0);
            });
    }

    fn render_history_list(&self, ui: &mut Ui) {
        use crate::history::store::SessionStatus;
        egui::ScrollArea::vertical()
            .auto_shrink([false, false])
            .show(ui, |ui| {
                if self.session_mgr.sessions.is_empty() {
                    ui.label(RichText::new("No sessions yet").color(TEXT_DIM).size(12.0).italics());
                    return;
                }
                for session in self.session_mgr.sessions.iter().rev() {
                    let (status_icon, status_color) = match session.status {
                        SessionStatus::Active  => ("[R]", ACCENT_GREEN),
                        SessionStatus::Closed  => ("[E]", TEXT_DIM),
                        SessionStatus::Crashed => ("[X]", ACCENT_RED),
                    };
                    ui.horizontal(|ui| {
                        ui.label(RichText::new(status_icon).color(status_color).size(10.0));
                        ui.label(RichText::new(&session.label).color(TEXT_PRIMARY).size(13.0));
                    });
                    if let Some(preview) = &session.first_input {
                        ui.label(
                            RichText::new(format!("  {preview}"))
                                .color(TEXT_DIM).size(11.0).italics()
                        );
                    }
                    ui.add_space(3.0);
                }
            });
    }

    // ── Agent panels (right area) ─────────────────────────────────────────────

    fn render_panels(&mut self, ctx: &egui::Context) {
        egui::CentralPanel::default()
            .frame(egui::Frame::none().fill(BG_DARK))
            .show(ctx, |ui| {
                if self.panel_mgr.panels().is_empty() {
                    self.render_empty_state(ui);
                    return;
                }

                self.handle_panel_shortcuts(ctx);

                match self.layout_mode() {
                    LayoutMode::Wide => self.render_panels_wide(ui, ctx),
                    LayoutMode::Portrait => self.render_panels_portrait(ui, ctx),
                }
            });
    }

    fn handle_panel_shortcuts(&mut self, ctx: &egui::Context) {
        if ctx.input(|i| i.key_pressed(egui::Key::N) && i.modifiers.ctrl) {
            self.show_new_panel_dialog = true;
        }
        if ctx.input(|i| i.key_pressed(egui::Key::W) && i.modifiers.ctrl) {
            if let Some(panel_id) = self.panel_mgr.focused_id() {
                self.panel_mgr.close_panel(panel_id);
            }
        }
        if ctx.input(|i| i.key_pressed(egui::Key::CloseBracket) && i.modifiers.ctrl) {
            self.panel_mgr.focus_next();
        }
        if ctx.input(|i| i.key_pressed(egui::Key::OpenBracket) && i.modifiers.ctrl) {
            self.panel_mgr.focus_prev();
        }
        if ctx.input(|i| i.key_pressed(egui::Key::Q) && i.modifiers.ctrl && i.modifiers.shift) {
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
        }
    }

    fn render_panels_wide(&mut self, ui: &mut Ui, ctx: &egui::Context) {
        let panel_ids: Vec<u32> = self.panel_mgr.panels().iter().map(|p| p.id).collect();
        let n = panel_ids.len();
        let available = ui.available_size();
        let panel_width = (available.x / n as f32).floor();

        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing = Vec2::ZERO;
            for panel_id in panel_ids {
                let (panel_rect, _) = ui.allocate_exact_size(
                    Vec2::new(panel_width, available.y),
                    egui::Sense::hover(),
                );
                let mut child_ui = ui.child_ui(panel_rect, *ui.layout());
                self.render_panel_card(&mut child_ui, ctx, panel_id);
            }
        });
    }

    fn render_panels_portrait(&mut self, ui: &mut Ui, ctx: &egui::Context) {
        let focused_id = self
            .panel_mgr
            .focused_id()
            .or_else(|| self.panel_mgr.panels().first().map(|panel| panel.id));

        if let Some(panel_id) = focused_id {
            self.panel_mgr.set_focus(panel_id);
            let available = ui.available_size();
            let (panel_rect, _) = ui.allocate_exact_size(available, egui::Sense::hover());
            let mut child_ui = ui.child_ui(panel_rect, *ui.layout());
            self.render_panel_card(&mut child_ui, ctx, panel_id);
        }
    }

    fn render_panel_card(&mut self, ui: &mut Ui, ctx: &egui::Context, panel_id: u32) {
        let focused = self.panel_mgr.focused_id() == Some(panel_id);
        let (label, status) = {
            let p = self.panel_mgr.panel(panel_id).unwrap();
            (p.label.clone(), p.status.clone())
        };

        let border = if focused { ACCENT } else { BORDER_COLOR };
        let panel_size = ui.available_size();
        let panel_frame = egui::Frame::none()
            .fill(BG_PANEL)
            .stroke(egui::Stroke::new(if focused { 1.5 } else { 1.0 }, border))
            .rounding(4.0)
            .inner_margin(egui::Margin::same(0.0));

        panel_frame.show(ui, |ui| {
            ui.set_min_size(panel_size);

            if ui
                .interact(ui.max_rect(), egui::Id::new(("panel_bg", panel_id)), egui::Sense::click())
                .clicked()
            {
                self.panel_mgr.set_focus(panel_id);
            }

            ui.vertical(|ui| {
                let header_bg = if focused {
                    Color32::from_rgb(28, 22, 55)
                } else {
                    Color32::from_rgb(20, 20, 32)
                };

                egui::Frame::none()
                    .fill(header_bg)
                    .inner_margin(egui::Margin::symmetric(8.0, 4.0))
                    .show(ui, |ui| {
                        ui.set_min_height(PANEL_HEADER_HEIGHT);
                        ui.set_max_height(PANEL_HEADER_HEIGHT);
                        ui.horizontal(|ui| {
                            let (dot, dot_color) = match &status {
                                PanelStatus::Starting => ("[+]", ACCENT_YELLOW),
                                PanelStatus::Running { .. } => ("[R]", ACCENT_GREEN),
                                PanelStatus::Exited { .. } => ("[E]", TEXT_DIM),
                                PanelStatus::Crashed { .. } => ("[X]", ACCENT_RED),
                            };
                            ui.label(RichText::new(dot).color(dot_color).size(11.0));
                            ui.label(
                                RichText::new(&label)
                                    .color(if focused { TEXT_PRIMARY } else { TEXT_DIM })
                                    .size(13.0)
                                    .strong()
                            );
                            if let PanelStatus::Running { pid } = &status {
                                ui.label(RichText::new(format!("pid:{pid}")).color(TEXT_DIM).size(11.0));
                            }
                            if ui.add(
                                egui::Button::new(RichText::new("X").color(ACCENT_RED).size(12.0))
                                    .fill(Color32::TRANSPARENT)
                                    .frame(false)
                            ).on_hover_text("Close panel")
                            .clicked() {
                                self.panel_mgr.close_panel(panel_id);
                            }
                        });
                    });

                ui.add(egui::Separator::default().horizontal().spacing(0.0));

                let output_height = ui.available_height() - PANEL_INPUT_HEIGHT;
                let output_size = Vec2::new(ui.available_width(), output_height.max(40.0));
                let (output_rect, _) = ui.allocate_exact_size(output_size, egui::Sense::hover());
                let (cols, rows) = terminal_grid_size(ctx, output_rect.size());
                self.panel_mgr.resize_panel(panel_id, cols, rows);

                let output_snapshot = self
                    .panel_mgr
                    .panel(panel_id)
                    .map(|p| p.terminal.lines().to_vec())
                    .unwrap_or_default();

                let mut output_ui = ui.child_ui(output_rect, egui::Layout::top_down(egui::Align::Min));
                output_ui.set_clip_rect(output_rect);

                egui::Frame::none()
                    .fill(BG_DARK)
                    .inner_margin(egui::Margin::symmetric(8.0, 4.0))
                    .show(&mut output_ui, |ui| {
                        ui.set_min_size(output_rect.size());
                        ui.spacing_mut().item_spacing.y = 0.0;

                        if output_snapshot.is_empty() {
                            ui.label(
                                RichText::new("Waiting for terminal output…")
                                    .color(TEXT_DIM)
                                    .size(12.0)
                                    .italics()
                            );
                        } else {
                            for line in &output_snapshot {
                                ui.label(layout_terminal_line(line));
                            }
                        }
                    });

                ui.add(egui::Separator::default().horizontal().spacing(0.0));

                egui::Frame::none()
                    .fill(BG_INPUT)
                    .inner_margin(egui::Margin::symmetric(8.0, 6.0))
                    .show(ui, |ui| {
                        ui.set_min_height(PANEL_INPUT_HEIGHT);
                        ui.set_max_height(PANEL_INPUT_HEIGHT);
                        let is_running = matches!(status, PanelStatus::Running { .. });
                        let status_text = if is_running {
                            "Focused panel sends keys directly to the PTY"
                        } else {
                            "Panel is not running"
                        };

                        ui.horizontal(|ui| {
                            ui.label(
                                RichText::new(">")
                                    .color(if is_running { ACCENT_GREEN } else { TEXT_DIM })
                                    .size(15.0)
                            );
                            ui.label(
                                RichText::new(status_text)
                                    .color(TEXT_PRIMARY)
                                    .size(12.0)
                            );
                            ui.label(
                                RichText::new("Ctrl+C = SIGINT")
                                    .color(TEXT_DIM)
                                    .size(11.0)
                            );
                        });
                    });
            });
        });
    }

    fn render_empty_state(&mut self, ui: &mut Ui) {
        ui.vertical_centered(|ui| {
            ui.add_space(ui.available_height() / 3.0);

            ui.add_space(12.0);
            ui.label(RichText::new("VibingIDE").size(28.0).strong().color(TEXT_PRIMARY));
            ui.add_space(6.0);
            ui.label(RichText::new("Agent-First IDE").size(14.0).color(TEXT_DIM));
            ui.add_space(28.0);

            if ui.add(
                egui::Button::new(
                    RichText::new("  +  Start New Agent Panel  ")
                        .size(16.0)
                        .color(Color32::WHITE)
                        .strong()
                )
                .fill(ACCENT)
                .rounding(10.0)
                .min_size(Vec2::new(260.0, 50.0))
            ).clicked() {
                self.show_new_panel_dialog = true;
            }

            ui.add_space(16.0);
            ui.label(
                RichText::new("Tip: launch  claude  /  opencode  /  any CLI tool")
                    .size(12.0)
                    .color(TEXT_DIM)
                    .italics()
            );
        });
    }
}

impl eframe::App for VibingApp {
    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        eframe::set_value(storage, PERSISTED_UI_KEY, &self.persisted_ui_state());
    }

    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // 1. Drain PTY events (non-blocking)
        self.drain_pty_events();
        self.handle_zoom_shortcuts(ctx);
        self.apply_zoom(ctx);
        let next_layout_mode = layout_mode_for_size_with_hysteresis(
            viewport_size_for_layout(ctx),
            self.layout_mode,
        );
        if next_layout_mode != self.layout_mode {
            debug!(
                ?self.layout_mode,
                ?next_layout_mode,
                viewport_size = ?viewport_size_for_layout(ctx),
                "Layout mode changed"
            );
        }
        self.layout_mode = next_layout_mode;
        self.handle_terminal_input(ctx);

        // 2. Render Shell & Workspace
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
            });

        // 3. Overlays
        if self.show_help             { self.show_help_window(ctx); }
        if self.show_new_panel_dialog { self.show_new_panel_dialog(ctx); }

        // 4. Request repaint at ~60fps for live PTY output
        ctx.request_repaint_after(std::time::Duration::from_millis(16));
    }
}

// ── Helpers ───────────────────────────────────────────────────────────────────

fn layout_terminal_line(line: &crate::pty::ansi::StyledLine) -> egui::text::LayoutJob {
    let mut job = egui::text::LayoutJob::default();
    job.wrap.break_anywhere = false;
    job.wrap.max_width = f32::INFINITY;

    if line.cells.is_empty() {
        job.append(
            " ",
            0.0,
            egui::text::TextFormat {
                font_id: FontId::monospace(13.0),
                color: TEXT_PRIMARY,
                ..Default::default()
            },
        );
        return job;
    }

    let mut current_text = String::new();
    let mut current_style = line.cells[0].style;

    for cell in &line.cells {
        if cell.style != current_style {
            push_terminal_run(&mut job, &current_text, current_style);
            current_text.clear();
            current_style = cell.style;
        }
        current_text.push(cell.ch);
    }

    if !current_text.is_empty() {
        push_terminal_run(&mut job, &current_text, current_style);
    }

    job
}

fn push_terminal_run(job: &mut egui::text::LayoutJob, text: &str, style: crate::pty::ansi::CellStyle) {
    let format = egui::text::TextFormat {
        font_id: FontId::monospace(13.0),
        color: style.fg.unwrap_or(TEXT_PRIMARY),
        background: style.bg.unwrap_or(Color32::TRANSPARENT),
        italics: style.text.italics(),
        underline: if style.text.underline {
            egui::Stroke::new(1.0, style.fg.unwrap_or(TEXT_PRIMARY))
        } else {
            egui::Stroke::NONE
        },
        strikethrough: if style.text.strikethrough {
            egui::Stroke::new(1.0, style.fg.unwrap_or(TEXT_PRIMARY))
        } else {
            egui::Stroke::NONE
        },
        ..Default::default()
    };
    job.append(text, 0.0, format);
}

fn layout_mode_for_size(size: Vec2) -> LayoutMode {
    if size.x <= PORTRAIT_BREAKPOINT || size.y > size.x * PORTRAIT_ASPECT_THRESHOLD {
        LayoutMode::Portrait
    } else {
        LayoutMode::Wide
    }
}

fn layout_mode_for_size_with_hysteresis(size: Vec2, previous: LayoutMode) -> LayoutMode {
    match previous {
        LayoutMode::Wide => layout_mode_for_size(size),
        LayoutMode::Portrait => {
            if size.x > PORTRAIT_EXIT_BREAKPOINT
                && size.y <= size.x * PORTRAIT_EXIT_ASPECT_THRESHOLD
            {
                LayoutMode::Wide
            } else {
                LayoutMode::Portrait
            }
        }
    }
}

fn viewport_size_for_layout(ctx: &egui::Context) -> Vec2 {
    let zoom_factor = ctx.zoom_factor().max(f32::EPSILON);
    ctx.input(|i| {
        i.viewport()
            .inner_rect
            .map(|rect| rect.size())
            .unwrap_or_else(|| i.screen_rect().size())
    }) * zoom_factor
}

fn auto_zoom_factor_for_size(size: Vec2) -> f32 {
    let width_scale = size.x / BASE_WIDTH;
    let height_scale = size.y / BASE_HEIGHT;

    width_scale
        .min(height_scale)
        .clamp(MIN_AUTO_SCALE, MAX_AUTO_SCALE)
}

fn normalized_zoom_factor(zoom_factor: f32) -> f32 {
    (zoom_factor * 100.0).round() / 100.0
}

fn terminal_grid_size(ctx: &egui::Context, size: Vec2) -> (u16, u16) {
    let font_id = FontId::monospace(13.0);
    let (char_width, row_height) = ctx.fonts(|fonts| {
        let width = fonts.glyph_width(&font_id, 'W').max(7.0);
        let height = fonts.row_height(&font_id).max(14.0);
        (width, height)
    });

    let cols = (size.x / char_width).floor().max(8.0) as u16;
    let rows = (size.y / row_height).floor().max(2.0) as u16;
    (cols, rows)
}

fn terminal_bytes_for_event(event: &Event, application_cursor: bool) -> Option<Vec<u8>> {
    match event {
        Event::Text(text) if !text.is_empty() => Some(text.as_bytes().to_vec()),
        Event::Paste(text) if !text.is_empty() => Some(text.as_bytes().to_vec()),
        Event::Key {
            key,
            pressed: true,
            modifiers,
            ..
        } => {
            if is_reserved_app_shortcut(*key, *modifiers) {
                return None;
            }

            if modifiers.ctrl {
                if let Some(byte) = ctrl_byte_for_key(*key) {
                    return Some(vec![byte]);
                }
            }

            key_to_terminal_bytes(*key, *modifiers, application_cursor)
        }
        _ => None,
    }
}

fn is_reserved_app_shortcut(key: Key, modifiers: Modifiers) -> bool {
    if modifiers.command && matches!(key, Key::Plus | Key::Equals | Key::Minus | Key::Num0) {
        return true;
    }

    if modifiers.ctrl && matches!(key, Key::N | Key::W | Key::OpenBracket | Key::CloseBracket) {
        return true;
    }

    modifiers.ctrl && modifiers.shift && key == Key::Q
}

fn ctrl_byte_for_key(key: Key) -> Option<u8> {
    use Key::*;
    let letter = match key {
        A => b'a',
        B => b'b',
        C => b'c',
        D => b'd',
        E => b'e',
        F => b'f',
        G => b'g',
        H => b'h',
        I => b'i',
        J => b'j',
        K => b'k',
        L => b'l',
        M => b'm',
        N => b'n',
        O => b'o',
        P => b'p',
        Q => b'q',
        R => b'r',
        S => b's',
        T => b't',
        U => b'u',
        V => b'v',
        W => b'w',
        X => b'x',
        Y => b'y',
        Z => b'z',
        _ => return None,
    };

    Some(letter - b'a' + 1)
}

fn key_to_terminal_bytes(key: Key, modifiers: Modifiers, application_cursor: bool) -> Option<Vec<u8>> {
    if modifiers.ctrl {
        return None;
    }

    let bytes = match key {
        Key::Enter => b"\r".to_vec(),
        Key::Tab => {
            if modifiers.shift {
                b"\x1b[Z".to_vec()
            } else {
                b"\t".to_vec()
            }
        }
        Key::Backspace => vec![0x7f],
        Key::Escape => vec![0x1b],
        Key::ArrowUp => cursor_key_bytes(application_cursor, b'A'),
        Key::ArrowDown => cursor_key_bytes(application_cursor, b'B'),
        Key::ArrowRight => cursor_key_bytes(application_cursor, b'C'),
        Key::ArrowLeft => cursor_key_bytes(application_cursor, b'D'),
        Key::Home => b"\x1b[H".to_vec(),
        Key::End => b"\x1b[F".to_vec(),
        Key::Insert => b"\x1b[2~".to_vec(),
        Key::Delete => b"\x1b[3~".to_vec(),
        Key::PageUp => b"\x1b[5~".to_vec(),
        Key::PageDown => b"\x1b[6~".to_vec(),
        _ => return None,
    };

    Some(bytes)
}

fn cursor_key_bytes(application_cursor: bool, final_byte: u8) -> Vec<u8> {
    if application_cursor {
        vec![0x1b, b'O', final_byte]
    } else {
        vec![0x1b, b'[', final_byte]
    }
}

fn tab_button(label: &str, active: bool) -> impl egui::Widget + '_ {
    move |ui: &mut Ui| {
        let fill = if active {
            ACCENT
        } else {
            Color32::TRANSPARENT
        };
        let text_color = if active { Color32::WHITE } else { TEXT_DIM };
        ui.add(
            egui::Button::new(RichText::new(label).size(12.0).color(text_color))
                .fill(fill)
                .rounding(6.0)
                .min_size(Vec2::new(80.0, 26.0))
        )
    }
}

fn render_nodes(ui: &mut Ui, nodes: &[crate::engine::project::FileNode], depth: usize) {
    for node in nodes {
        let indent = depth as f32 * 14.0;
        ui.horizontal(|ui| {
            ui.add_space(indent + 4.0);
            let icon  = if node.is_dir() { "+" } else { " " };
            let color = if node.is_dir() { ACCENT_YELLOW } else { TEXT_PRIMARY };
            let text  = RichText::new(format!("{} {}", icon, node.name))
                .size(12.5)
                .color(color);
            let response = ui.add(egui::Label::new(text).sense(egui::Sense::click()));
            if response.hovered() {
                response.on_hover_cursor(egui::CursorIcon::PointingHand);
            }
        });
        if node.is_dir() && !node.children.is_empty() {
            render_nodes(ui, &node.children, depth + 1);
        }
    }
}

// Trait extension helper
trait TextStyleExt {
    fn italics(&self) -> bool;
}
impl TextStyleExt for crate::pty::ansi::TextStyle {
    fn italics(&self) -> bool { self.italic }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn key_event(key: Key, modifiers: Modifiers) -> Event {
        Event::Key {
            key,
            physical_key: None,
            pressed: true,
            repeat: false,
            modifiers,
        }
    }

    #[test]
    fn text_and_paste_events_pass_through_to_terminal() {
        assert_eq!(
            terminal_bytes_for_event(&Event::Text("codex".into()), false),
            Some(b"codex".to_vec())
        );
        assert_eq!(
            terminal_bytes_for_event(&Event::Paste("hello\nworld".into()), false),
            Some(b"hello\nworld".to_vec())
        );
    }

    #[test]
    fn ctrl_c_maps_to_interrupt_byte_but_quit_shortcuts_stay_reserved() {
        let ctrl = Modifiers {
            ctrl: true,
            ..Default::default()
        };
        let ctrl_shift = Modifiers {
            ctrl: true,
            shift: true,
            ..Default::default()
        };

        assert_eq!(
            terminal_bytes_for_event(&key_event(Key::C, ctrl), false),
            Some(vec![0x03])
        );
        assert_eq!(
            terminal_bytes_for_event(&key_event(Key::Q, ctrl_shift), false),
            None
        );
        assert_eq!(
            terminal_bytes_for_event(&key_event(Key::W, ctrl), false),
            None
        );
    }

    #[test]
    fn arrow_keys_follow_application_cursor_mode() {
        assert_eq!(
            terminal_bytes_for_event(&key_event(Key::ArrowUp, Modifiers::default()), false),
            Some(b"\x1b[A".to_vec())
        );
        assert_eq!(
            terminal_bytes_for_event(&key_event(Key::ArrowUp, Modifiers::default()), true),
            Some(b"\x1bOA".to_vec())
        );
        assert_eq!(
            terminal_bytes_for_event(&key_event(Key::Tab, Modifiers::default()), false),
            Some(b"\t".to_vec())
        );
    }

    #[test]
    fn portrait_layout_is_selected_for_tall_or_narrow_windows() {
        assert_eq!(layout_mode_for_size(Vec2::new(480.0, 960.0)), LayoutMode::Portrait);
        assert_eq!(layout_mode_for_size(Vec2::new(820.0, 700.0)), LayoutMode::Portrait);
        assert_eq!(layout_mode_for_size(Vec2::new(1280.0, 800.0)), LayoutMode::Wide);
    }

    #[test]
    fn portrait_layout_hysteresis_prevents_threshold_flapping() {
        assert_eq!(
            layout_mode_for_size_with_hysteresis(Vec2::new(895.0, 700.0), LayoutMode::Wide),
            LayoutMode::Portrait
        );
        assert_eq!(
            layout_mode_for_size_with_hysteresis(Vec2::new(905.0, 700.0), LayoutMode::Portrait),
            LayoutMode::Portrait
        );
        assert_eq!(
            layout_mode_for_size_with_hysteresis(Vec2::new(960.0, 700.0), LayoutMode::Portrait),
            LayoutMode::Wide
        );
    }

    #[test]
    fn auto_zoom_uses_unzoomed_viewport_size() {
        let raw_zoomed_screen = Vec2::new(1280.0 / 1.3, 800.0 / 1.3);
        let recovered_viewport = raw_zoomed_screen * 1.3;

        assert_eq!(recovered_viewport, Vec2::new(1280.0, 800.0));
        assert_eq!(auto_zoom_factor_for_size(recovered_viewport), 1.0);
    }

    #[test]
    fn auto_zoom_does_not_upscale_large_windows() {
        assert_eq!(auto_zoom_factor_for_size(Vec2::new(1600.0, 1000.0)), 1.0);
        assert_eq!(auto_zoom_factor_for_size(Vec2::new(1920.0, 1080.0)), 1.0);
    }

    #[test]
    fn zoom_factor_normalization_removes_tiny_jitter() {
        assert_eq!(normalized_zoom_factor(1.279_999), 1.28);
        assert_eq!(normalized_zoom_factor(1.280_001), 1.28);
    }
}





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
    }
}

