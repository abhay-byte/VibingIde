//! VibingIDE — egui Application
//!
//! Implements `eframe::App`. Owns all state; communicates with PTY supervisor
//! tasks via a tokio mpsc channel polled synchronously in `update()`.

#![allow(dead_code, unused_variables)]

use std::path::PathBuf;
use std::sync::Arc;

use egui::{Color32, Event, FontId, Key, KeyboardShortcut, Modifiers, RichText, Ui, Vec2};
use tokio::sync::mpsc;
use tracing::warn;

use crate::config::AppConfig;
use crate::engine::{
    panel_manager::{PanelManager, PanelStatus},
    project::Project,
    session_manager::SessionManager,
};
use crate::pty::supervisor::PtyEvent;

// ── Design tokens ──────────────────────────────────────────────────────────────

const BG_DARK:       Color32 = Color32::from_rgb(14,  14,  22);
const BG_PANEL:      Color32 = Color32::from_rgb(22,  22,  34);
const BG_SIDEBAR:    Color32 = Color32::from_rgb(18,  18,  28);
const BG_INPUT:      Color32 = Color32::from_rgb(28,  28,  42);
const BORDER_COLOR:  Color32 = Color32::from_rgb(50,  50,  78);
const ACCENT:        Color32 = Color32::from_rgb(120, 87,  255);
const ACCENT_GREEN:  Color32 = Color32::from_rgb(52,  211, 153);
const ACCENT_RED:    Color32 = Color32::from_rgb(239, 68,  68);
const ACCENT_YELLOW: Color32 = Color32::from_rgb(251, 191, 36);
const TEXT_PRIMARY:  Color32 = Color32::from_rgb(225, 225, 235);
const TEXT_DIM:      Color32 = Color32::from_rgb(110, 110, 140);
const TEXT_MONO:     &str    = "Consolas, Monaco, Courier New";
const ZOOM_STEP:     f32     = 0.1;
const MIN_ZOOM:      f32     = 0.7;
const MAX_ZOOM:      f32     = 2.0;
const BASE_WIDTH:    f32     = 1280.0;
const BASE_HEIGHT:   f32     = 800.0;
const MIN_AUTO_SCALE:f32     = 0.85;
const MAX_AUTO_SCALE:f32     = 1.35;

// ── App state ──────────────────────────────────────────────────────────────────

/// Which view is active in the left sidebar.
#[derive(PartialEq, Clone)]
enum SidebarView { Files, History }

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
        _cc:         &eframe::CreationContext,
        project_root: PathBuf,
        config:       AppConfig,
        initial_cmd:  Option<String>,
        rt:           Arc<tokio::runtime::Runtime>,
    ) -> Self {
        let auto_scale = config.ui.auto_scale;
        let manual_zoom_factor = config.ui.zoom_factor;
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
        }
    }

    /// Configure the egui visual style.
    pub fn setup_visuals(ctx: &egui::Context) {
        let mut visuals = egui::Visuals::dark();
        visuals.window_fill         = BG_DARK;
        visuals.panel_fill          = BG_PANEL;
        visuals.faint_bg_color      = BG_SIDEBAR;
        visuals.extreme_bg_color    = BG_INPUT;
        visuals.window_rounding     = egui::Rounding::same(8.0);
        visuals.widgets.noninteractive.bg_fill = BG_PANEL;
        visuals.widgets.inactive.bg_fill       = BG_INPUT;
        visuals.widgets.hovered.bg_fill        = Color32::from_rgb(60, 60, 90);
        visuals.widgets.active.bg_fill         = ACCENT;
        visuals.selection.bg_fill              = Color32::from_rgb(80, 60, 200);
        ctx.set_visuals(visuals);

        let fonts = egui::FontDefinitions::default();
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

        let size = ctx.input(|i| i.screen_rect().size());
        let width_scale = size.x / BASE_WIDTH;
        let height_scale = size.y / BASE_HEIGHT;

        width_scale
            .min(height_scale)
            .clamp(MIN_AUTO_SCALE, MAX_AUTO_SCALE)
    }

    fn apply_zoom(&mut self, ctx: &egui::Context) {
        let target_zoom = (self.manual_zoom_factor * self.auto_zoom_factor(ctx))
            .clamp(MIN_ZOOM, MAX_ZOOM);

        if (ctx.zoom_factor() - target_zoom).abs() > f32::EPSILON {
            ctx.set_zoom_factor(target_zoom);
        }

        self.applied_zoom_factor = target_zoom;
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
        egui::TopBottomPanel::top("toolbar")
            .exact_height(42.0)
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

                ui.horizontal_centered(|ui| {
                    // Logo
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
                        // Window controls (Close)
                        if ui.add(
                            egui::Button::new(RichText::new(" ✕ ").color(Color32::WHITE).size(13.0))
                                .fill(Color32::from_rgb(160, 40, 40))
                                .rounding(6.0)
                        ).clicked() {
                            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                        }
                        
                        // Window controls (Maximize)
                        if ui.add(
                            egui::Button::new(RichText::new(" □ ").color(TEXT_PRIMARY).size(13.0))
                                .fill(Color32::from_rgb(40, 40, 65))
                                .rounding(6.0)
                        ).clicked() {
                            let is_max = ctx.input(|i| i.viewport().maximized.unwrap_or(false));
                            ctx.send_viewport_cmd(egui::ViewportCommand::Maximized(!is_max));
                        }

                        // Window controls (Minimize)
                        if ui.add(
                            egui::Button::new(RichText::new(" _ ").color(TEXT_PRIMARY).size(13.0))
                                .fill(Color32::from_rgb(40, 40, 65))
                                .rounding(6.0)
                        ).clicked() {
                            ctx.send_viewport_cmd(egui::ViewportCommand::Minimized(true));
                        }
                        ui.add_space(12.0);

                        ui.label(
                            RichText::new(format!("{:.0}% zoom", self.applied_zoom_factor * 100.0))
                                .color(TEXT_DIM)
                                .size(12.0)
                        );
                        ui.add_space(8.0);

                        // Help toggle
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

                        // New Panel
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
            });
    }

    // ── Left sidebar ──────────────────────────────────────────────────────────

    fn render_sidebar(&mut self, ctx: &egui::Context) {
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

                // Global keyboard shortcuts
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

                let panel_ids: Vec<u32> = self.panel_mgr.panels().iter().map(|p| p.id).collect();
                let n = panel_ids.len();
                let available = ui.available_size();
                let panel_width = (available.x / n as f32).floor();

                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing = Vec2::ZERO;

                    for panel_id in panel_ids {
                        let focused = self.panel_mgr.focused_id() == Some(panel_id);

                        // Collect needed info before mutable borrow
                        let (label, status) = {
                            let p = self.panel_mgr.panel(panel_id).unwrap();
                            (
                                p.label.clone(),
                                p.status.clone(),
                            )
                        };

                        let border = if focused { ACCENT } else { BORDER_COLOR };
                        let panel_frame = egui::Frame::none()
                            .fill(BG_PANEL)
                            .stroke(egui::Stroke::new(if focused { 1.5 } else { 1.0 }, border))
                            .rounding(4.0)
                            .inner_margin(egui::Margin::same(0.0));

                        // Layout slot
                        let (panel_rect, _) = ui.allocate_exact_size(
                            Vec2::new(panel_width, available.y),
                            egui::Sense::hover(),
                        );

                        let mut child_ui = ui.child_ui(panel_rect, *ui.layout());

                        panel_frame.show(&mut child_ui, |ui| {
                            // Click anywhere to focus
                            if ui.interact(ui.max_rect(), egui::Id::new(("panel_bg", panel_id)), egui::Sense::click()).clicked() {
                                self.panel_mgr.set_focus(panel_id);
                            }

                            ui.vertical(|ui| {
                                // ── Header bar ──────────────────────────────
                                let header_bg = if focused {
                                    Color32::from_rgb(28, 22, 55)
                                } else {
                                    Color32::from_rgb(20, 20, 32)
                                };
                                egui::Frame::none()
                                    .fill(header_bg)
                                    .inner_margin(egui::Margin::symmetric(8.0, 4.0))
                                    .show(ui, |ui| {
                                        ui.horizontal(|ui| {
                                            // Status dot
                                            let (dot, dot_color) = match &status {
                                                PanelStatus::Starting       => ("[+]", ACCENT_YELLOW),
                                                PanelStatus::Running { .. } => ("[R]",  ACCENT_GREEN),
                                                PanelStatus::Exited  { .. } => ("[E]",  TEXT_DIM),
                                                PanelStatus::Crashed { .. } => ("[X]",  ACCENT_RED),
                                            };
                                            ui.label(RichText::new(dot).color(dot_color).size(11.0));

                                            // Panel label
                                            ui.label(
                                                RichText::new(&label)
                                                    .color(if focused { TEXT_PRIMARY } else { TEXT_DIM })
                                                    .size(13.0)
                                                    .strong()
                                            );

                                            // PID info
                                            if let PanelStatus::Running { pid } = &status {
                                                ui.label(RichText::new(format!("pid:{pid}")).color(TEXT_DIM).size(11.0));
                                            }

                                            // Close button on right
                                            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                                if ui.add(
                                                    egui::Button::new(RichText::new("X").color(ACCENT_RED).size(12.0))
                                                        .fill(Color32::TRANSPARENT)
                                                        .frame(false)
                                                        .sense(egui::Sense::click())
                                                ).on_hover_text("Close panel")
                                                .clicked() {
                                                    self.panel_mgr.close_panel(panel_id);
                                                }
                                            });
                                        });
                                    });

                                ui.add(egui::Separator::default().horizontal().spacing(0.0));

                                // ── Output area ─────────────────────────────
                                let output_height = ui.available_height() - 28.0;
                                let output_frame = egui::Frame::none()
                                    .fill(BG_DARK)
                                    .inner_margin(egui::Margin::symmetric(8.0, 4.0));

                                output_frame.show(ui, |ui| {
                                    ui.set_min_height(output_height.max(40.0));
                                    ui.set_max_height(output_height.max(40.0));
                                    let (cols, rows) = terminal_grid_size(ctx, ui.available_size());
                                    self.panel_mgr.resize_panel(panel_id, cols, rows);

                                    let output_snapshot = self
                                        .panel_mgr
                                        .panel(panel_id)
                                        .map(|p| p.terminal.lines().to_vec())
                                        .unwrap_or_default();

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

                                // ── Terminal status bar ─────────────────────
                                ui.add(egui::Separator::default().horizontal().spacing(0.0));

                                egui::Frame::none()
                                    .fill(BG_INPUT)
                                    .inner_margin(egui::Margin::symmetric(8.0, 6.0))
                                    .show(ui, |ui| {
                                        let is_running = matches!(status, PanelStatus::Running { .. });
                                        let status_text = if is_running {
                                            "Direct terminal mode: type directly into the focused panel"
                                        } else {
                                            "Panel is not running"
                                        };

                                        ui.horizontal_wrapped(|ui| {
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
                                                RichText::new("Ctrl+C sends SIGINT. Ctrl+Shift+Q quits VibingIDE.")
                                                    .color(TEXT_DIM)
                                                    .size(11.0)
                                            );
                                        });
                                    });
                            });
                        });
                    }
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
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // 1. Drain PTY events (non-blocking)
        self.drain_pty_events();
        self.handle_zoom_shortcuts(ctx);
        self.apply_zoom(ctx);
        self.handle_terminal_input(ctx);

        // 2. Render UI
        self.render_toolbar(ctx);
        self.render_sidebar(ctx);
        self.render_panels(ctx);

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
}
