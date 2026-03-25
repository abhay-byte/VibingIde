//! Keybinding help overlay modal.

use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::Span,
    widgets::{Block, BorderType, Borders, Clear, Row, Table},
    Frame,
};

use crate::input::keybinds::help_entries;

/// Render a centered popup showing all keybindings.
/// Clears the area behind it first so it floats above the main UI.
pub fn render_keybind_overlay(frame: &mut Frame) {
    let area = centered_rect(60, 80, frame.size());

    // Clear the area behind the popup
    frame.render_widget(Clear, area);

    let block = Block::default()
        .title(Span::styled(
            " ⌨  Keybindings ",
            Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD),
        ))
        .borders(Borders::ALL)
        .border_type(BorderType::Double)
        .border_style(Style::default().fg(Color::Cyan));

    let inner = block.inner(area);
    frame.render_widget(block, area);

    let rows: Vec<Row> = help_entries()
        .iter()
        .map(|(key, desc)| {
            Row::new(vec![
                ratatui::text::Text::raw(*key),
                ratatui::text::Text::raw(*desc),
            ])
            .style(Style::default().fg(Color::White))
        })
        .collect();

    let widths = [Constraint::Length(18), Constraint::Min(20)];
    let table = Table::new(rows, widths)
        .header(
            Row::new(vec!["Key", "Action"])
                .style(Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
        )
        .column_spacing(2);

    frame.render_widget(table, inner);
}

/// Compute a centered rectangle that is `percent_x`% wide and `percent_y`% tall.
fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(r);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_layout[1])[1]
}
