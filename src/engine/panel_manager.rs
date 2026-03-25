//! Panel manager — CRUD for agent panels, focus tracking.

use std::collections::VecDeque;
use std::path::PathBuf;

use anyhow::Result;
use portable_pty::PtySize;
use tokio::sync::mpsc;
use tracing::info;

use crate::pty::ansi::StyledLine;
use crate::pty::supervisor::{PanelId, PtyEvent, Supervisor};

// ── Panel data ────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq)]
pub enum PanelStatus {
    Starting,
    Running { pid: u32 },
    Exited  { code: i32 },
    Crashed { signal: Option<i32> },
}

pub struct AgentPanel {
    pub id:         PanelId,
    pub label:      String,
    pub command:    String,
    pub args:       Vec<String>,
    pub status:     PanelStatus,
    pub session_id: String,
    /// Output ring buffer (max 10k lines).
    pub output_buf: VecDeque<StyledLine>,
    /// Current text in input bar.
    pub input_buf:  String,
    /// Lines scrolled up from bottom (0 = at bottom).
    pub scroll_pos: usize,
}

impl AgentPanel {
    pub fn new(
        id: PanelId,
        label: String,
        command: String,
        args: Vec<String>,
        session_id: String,
    ) -> Self {
        Self {
            id,
            label,
            command,
            args,
            status:     PanelStatus::Starting,
            session_id,
            output_buf: VecDeque::with_capacity(1024),
            input_buf:  String::new(),
            scroll_pos: 0,
        }
    }

    /// Push lines into the output ring buffer, capping at `max_lines`.
    pub fn push_lines(&mut self, lines: Vec<StyledLine>, max_lines: usize) {
        for line in lines {
            if self.output_buf.len() >= max_lines {
                self.output_buf.pop_front();
            }
            self.output_buf.push_back(line);
        }
    }

    pub fn scroll_up(&mut self, n: usize) {
        let max = self.output_buf.len().saturating_sub(1);
        self.scroll_pos = (self.scroll_pos + n).min(max);
    }

    pub fn scroll_down(&mut self, n: usize) {
        self.scroll_pos = self.scroll_pos.saturating_sub(n);
    }

    pub fn scroll_to_bottom(&mut self) {
        self.scroll_pos = 0;
    }
}

// ── Panel manager ─────────────────────────────────────────────────────────────

pub struct PanelManager {
    panels:        Vec<AgentPanel>,
    supervisors:   Vec<Option<Supervisor>>,
    next_id:       PanelId,
    focused:       Option<PanelId>,
    pub event_tx:  mpsc::UnboundedSender<PtyEvent>,
    max_buf_lines: usize,
    env_allowlist: Vec<String>,
    cwd:           PathBuf,
}

impl PanelManager {
    pub fn new(
        event_tx:      mpsc::UnboundedSender<PtyEvent>,
        max_buf_lines: usize,
        env_allowlist: Vec<String>,
        cwd:           PathBuf,
    ) -> Self {
        Self {
            panels:        Vec::new(),
            supervisors:   Vec::new(),
            next_id:       0,
            focused:       None,
            event_tx,
            max_buf_lines,
            env_allowlist,
            cwd,
        }
    }

    /// Create and spawn a new agent panel. Returns the new PanelId.
    pub fn create_panel(
        &mut self,
        command:    String,
        args:       Vec<String>,
        session_id: String,
        term_size:  (u16, u16),
    ) -> Result<PanelId> {
        let id    = self.next_id;

        let label = format!("{} #{}", &command, id + 1);

        let size = PtySize {
            rows: term_size.1.max(24),
            cols: term_size.0.max(80),
            pixel_width:  0,
            pixel_height: 0,
        };

        // Spawn PTY first
        let supervisor = Supervisor::spawn(
            id,
            &command,
            &args,
            &self.cwd,
            &self.env_allowlist,
            size,
            self.event_tx.clone(),
        )?;

        // Only commit panel to state if spawn succeeded
        self.next_id += 1;
        let mut panel = AgentPanel::new(id, label, command.clone(), args.clone(), session_id);
        panel.status = PanelStatus::Running {
            pid: supervisor.process_id().unwrap_or(0),
        };
        self.panels.push(panel);
        self.supervisors.push(Some(supervisor));
        self.focused = Some(id);
        info!(panel_id = id, command, "Panel created");
        Ok(id)
    }

    /// Send input text to a panel's PTY stdin.
    pub fn send_input(&mut self, panel_id: PanelId, text: &str) -> Result<()> {
        if let Some(idx) = self.panel_index(panel_id) {
            if let Some(Some(sup)) = self.supervisors.get_mut(idx) {
                return sup.send_input(text);
            }
        }
        Ok(())
    }

    /// Close (kill) a panel by ID.
    pub fn close_panel(&mut self, panel_id: PanelId) {
        if let Some(idx) = self.panel_index(panel_id) {
            if let Some(Some(supervisor)) = self.supervisors.get_mut(idx) {
                let _ = supervisor.kill();
            }
            self.supervisors[idx] = None;
            if let Some(p) = self.panels.get_mut(idx) {
                p.status = PanelStatus::Exited { code: -1 };
            }
            self.focused = self.panels.iter()
                .find(|p| matches!(p.status, PanelStatus::Running { .. } | PanelStatus::Starting))
                .map(|p| p.id);
        }
    }

    pub fn handle_output(
        &mut self,
        panel_id:    PanelId,
        ansi_parser: &mut crate::pty::ansi::AnsiParser,
        data:        Vec<u8>,
    ) {
        let lines = ansi_parser.feed(&data);
        let max   = self.max_buf_lines;
        if let Some(panel) = self.panel_mut(panel_id) {
            panel.push_lines(lines, max);
        }
    }

    pub fn handle_exit(&mut self, panel_id: PanelId, exit_code: Option<i32>) {
        if let Some(idx) = self.panel_index(panel_id) {
            self.supervisors[idx] = None;
        }
        if let Some(panel) = self.panel_mut(panel_id) {
            panel.status = match exit_code {
                Some(0) => PanelStatus::Exited  { code: 0 },
                Some(c) => PanelStatus::Exited  { code: c },
                None    => PanelStatus::Crashed { signal: None },
            };
        }
    }

    pub fn focus_next(&mut self) {
        if self.panels.is_empty() { return; }
        let cur  = self.focused_index().unwrap_or(0);
        let next = (cur + 1) % self.panels.len();
        self.focused = Some(self.panels[next].id);
    }

    pub fn focus_prev(&mut self) {
        if self.panels.is_empty() { return; }
        let cur  = self.focused_index().unwrap_or(0);
        let prev = if cur == 0 { self.panels.len() - 1 } else { cur - 1 };
        self.focused = Some(self.panels[prev].id);
    }

    pub fn set_focus(&mut self, panel_id: PanelId) {
        if self.panels.iter().any(|p| p.id == panel_id) {
            self.focused = Some(panel_id);
        }
    }

    pub fn focused_panel(&self) -> Option<&AgentPanel> {
        self.focused.and_then(|id| self.panels.iter().find(|p| p.id == id))
    }

    pub fn focused_panel_mut(&mut self) -> Option<&mut AgentPanel> {
        let id = self.focused?;
        self.panels.iter_mut().find(|p| p.id == id)
    }

    pub fn panels(&self)     -> &[AgentPanel] { &self.panels }
    pub fn focused_id(&self) -> Option<PanelId> { self.focused }

    pub fn resize_all(&self, cols: u16, rows: u16) {
        let n = self.supervisors.iter().filter(|s| s.is_some()).count().max(1);
        let panel_rows = (rows / n as u16).max(2);
        for sup in self.supervisors.iter().flatten() {
            let _ = sup.resize(cols, panel_rows);
        }
    }

    // ── Helpers ───────────────────────────────────────────────────────────────

    fn panel_index(&self, id: PanelId) -> Option<usize> {
        self.panels.iter().position(|p| p.id == id)
    }

    fn focused_index(&self) -> Option<usize> {
        self.focused.and_then(|id| self.panel_index(id))
    }

    fn panel_mut(&mut self, id: PanelId) -> Option<&mut AgentPanel> {
        self.panels.iter_mut().find(|p| p.id == id)
    }
}
