//! Agent panel widget — renders one PTY session's output + input bar.

use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Paragraph, Wrap},
    Frame,
};

use crate::engine::panel_manager::{AgentPanel, PanelStatus};

/// Render a single agent panel into `area`.
/// `focused` is true when this panel has keyboard focus.
pub fn render_agent_panel(frame: &mut Frame, area: Rect, panel: &AgentPanel, focused: bool) {
    // ── Outer border ─────────────────────────────────────────────────────────
    let border_style = if focused {
        Style::default().fg(Color::Cyan)
    } else {
        Style::default().fg(Color::DarkGray)
    };

    let status_label = match &panel.status {
        PanelStatus::Starting     => "⏳ starting".to_string(),
        PanelStatus::Running { pid } => format!("● running PID {pid}"),
        PanelStatus::Exited { code }  => format!("✓ exited ({code})"),
        PanelStatus::Crashed { .. }   => "✗ crashed".to_string(),
    };

    let title = format!(" {} │ {} ", panel.label, status_label);
    let outer_block = Block::default()
        .borders(Borders::ALL)
        .border_type(if focused { BorderType::Thick } else { BorderType::Plain })
        .border_style(border_style)
        .title(Span::styled(title, Style::default().fg(Color::White).add_modifier(Modifier::BOLD)));

    let inner = outer_block.inner(area);
    frame.render_widget(outer_block, area);

    // ── Split inner into output viewport + input bar ──────────────────────────
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Min(3),      // output
            Constraint::Length(3),   // input bar
        ])
        .split(inner);

    render_output_viewport(frame, chunks[0], panel, focused);
    render_input_bar(frame, chunks[1], panel, focused);
}

fn render_output_viewport(frame: &mut Frame, area: Rect, panel: &AgentPanel, _focused: bool) {
    let visible_height = area.height as usize;
    let total_lines = panel.output_buf.len();

    // Compute which lines to show based on scroll_pos.
    let start = if total_lines > visible_height {
        let bottom_offset = panel.scroll_pos;
        total_lines
            .saturating_sub(visible_height)
            .saturating_sub(bottom_offset)
    } else {
        0
    };

    let lines_to_render: Vec<Line> = panel
        .output_buf
        .iter()
        .skip(start)
        .take(visible_height)
        .map(|sl| {
            let spans: Vec<Span> = sl
                .cells
                .iter()
                .map(|cell| Span::styled(cell.ch.to_string(), cell.style))
                .collect();
            Line::from(spans)
        })
        .collect();

    // Scroll indicator
    let scroll_note = if panel.scroll_pos > 0 {
        format!(" ↑ scroll +{} ", panel.scroll_pos)
    } else {
        String::new()
    };

    let para = Paragraph::new(lines_to_render)
        .block(
            Block::default()
                .title(Span::styled(scroll_note, Style::default().fg(Color::Yellow)))
                .borders(Borders::NONE),
        )
        .wrap(Wrap { trim: false });

    frame.render_widget(para, area);
}

fn render_input_bar(frame: &mut Frame, area: Rect, panel: &AgentPanel, focused: bool) {
    let is_active = focused; // input bar is active when panel has focus
    let border_style = if is_active {
        Style::default().fg(Color::Green)
    } else {
        Style::default().fg(Color::DarkGray)
    };

    let prompt = if matches!(panel.status, PanelStatus::Running { .. }) {
        "› "
    } else {
        "  "
    };

    let content = format!("{}{}", prompt, panel.input_buf);
    let input_para = Paragraph::new(content)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(border_style)
                .border_type(BorderType::Rounded),
        )
        .style(Style::default().fg(Color::White));

    frame.render_widget(input_para, area);
}
