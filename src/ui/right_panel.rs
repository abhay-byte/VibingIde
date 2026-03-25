//! Stub: renders right panel container (delegates to agent_panel per slot).

use ratatui::{Frame, layout::Rect};

use crate::engine::panel_manager::AgentPanel;
use crate::pty::supervisor::PanelId;
use crate::ui::{agent_panel::render_agent_panel, layout::split_right_panels};

pub fn render_right_panels(
    frame: &mut Frame,
    area: Rect,
    panels: &[AgentPanel],
    focused_id: Option<PanelId>,
) {
    if panels.is_empty() {
        use ratatui::{style::{Color, Style}, widgets::{Block, Borders, Paragraph}};
        let hint = Paragraph::new("Press Ctrl+Shift+N to start a new agent panel")
            .block(Block::default().borders(Borders::ALL).border_style(Style::default().fg(Color::DarkGray)))
            .style(Style::default().fg(Color::DarkGray));
        frame.render_widget(hint, area);
        return;
    }

    let rects = split_right_panels(area, panels.len());
    for (panel, rect) in panels.iter().zip(rects.iter()) {
        let focused = focused_id == Some(panel.id);
        render_agent_panel(frame, *rect, panel, focused);
    }
}
