//! Top toolbar — project name, quick-action buttons.
//!
//! Returns zone rects for mouse hit-testing.

use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Frame,
};

use crate::ui::hit_test::{ClickZone, Zone};

/// Render the toolbar and return click zones for its buttons.
///
/// Layout (left → right):
///   [logo/name]  ···spacer···  [⊕ New Panel]  [? Help]  [✕ Quit]
pub fn render_toolbar(
    frame: &mut Frame,
    area: Rect,
    project_name: &str,
    help_active: bool,
    hovered: Option<&ClickZone>,
) -> Vec<Zone> {
    // Background block
    let bg_block = Block::default()
        .borders(Borders::BOTTOM)
        .border_style(Style::default().fg(Color::Rgb(50, 50, 70)))
        .style(Style::default().bg(Color::Rgb(18, 18, 30)));
    frame.render_widget(bg_block, area);

    // Split toolbar: logo | spacer | buttons
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Min(20),       // logo / project name
            Constraint::Length(16),    // [⊕ New Panel]
            Constraint::Length(10),    // [? Help]
            Constraint::Length(9),     // [✕ Quit]
        ])
        .split(area);

    // ── Logo ─────────────────────────────────────────────────────────────────
    let logo = Paragraph::new(Line::from(vec![
        Span::styled("⚡ ", Style::default().fg(Color::Rgb(120, 100, 255))),
        Span::styled("VibingIDE", Style::default()
            .fg(Color::White)
            .add_modifier(Modifier::BOLD)),
        Span::styled(format!("  {project_name}"), Style::default().fg(Color::Rgb(100, 100, 130))),
    ]));
    frame.render_widget(logo, chunks[0]);

    // ── New Panel button ──────────────────────────────────────────────────────
    let new_hovered = hovered == Some(&ClickZone::ToolbarNewPanel);
    let new_style = button_style(new_hovered, false, Color::Rgb(80, 200, 120));
    let new_btn = Paragraph::new(Line::from(vec![
        Span::styled(" ⊕ New Panel ", new_style),
    ]));
    frame.render_widget(new_btn, chunks[1]);

    // ── Help button ───────────────────────────────────────────────────────────
    let help_hovered = hovered == Some(&ClickZone::ToolbarHelp);
    let help_style = button_style(help_hovered, help_active, Color::Rgb(100, 160, 255));
    let help_btn = Paragraph::new(Line::from(vec![
        Span::styled(" ? Help ", help_style),
    ]));
    frame.render_widget(help_btn, chunks[2]);

    // ── Quit button ───────────────────────────────────────────────────────────
    let quit_hovered = hovered == Some(&ClickZone::ToolbarQuit);
    let quit_style = button_style(quit_hovered, false, Color::Rgb(220, 80, 80));
    let quit_btn = Paragraph::new(Line::from(vec![
        Span::styled(" ✕ Quit ", quit_style),
    ]));
    frame.render_widget(quit_btn, chunks[3]);

    vec![
        Zone::new(ClickZone::ToolbarNewPanel, chunks[1]),
        Zone::new(ClickZone::ToolbarHelp,     chunks[2]),
        Zone::new(ClickZone::ToolbarQuit,     chunks[3]),
    ]
}

/// Compute a button's style based on hover and active state.
fn button_style(hovered: bool, active: bool, accent: Color) -> Style {
    if active {
        Style::default()
            .fg(Color::Black)
            .bg(accent)
            .add_modifier(Modifier::BOLD)
    } else if hovered {
        Style::default()
            .fg(accent)
            .add_modifier(Modifier::BOLD | Modifier::UNDERLINED)
    } else {
        Style::default()
            .fg(Color::Rgb(160, 160, 180))
    }
}
