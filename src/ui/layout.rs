//! Root layout: splits the terminal into left sidebar and right agent area.

use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    Frame,
};

/// Divide the terminal frame into left and right areas.
/// `left_pct` is a value 10–50 for the left panel width percentage.
pub fn split_root(frame: &Frame, left_pct: u8) -> (Rect, Rect) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(left_pct as u16),
            Constraint::Percentage(100 - left_pct as u16),
        ])
        .split(frame.size());

    (chunks[0], chunks[1])
}

/// Divide the right area into N equal vertical slices (one per panel).
/// Returns at most `n` rects; if n == 0 returns empty vec.
pub fn split_right_panels(area: Rect, n: usize) -> Vec<Rect> {
    if n == 0 {
        return vec![];
    }
    let constraints: Vec<Constraint> = (0..n)
        .map(|_| Constraint::Ratio(1, n as u32))
        .collect();

    Layout::default()
        .direction(Direction::Vertical)
        .constraints(constraints)
        .split(area)
        .iter()
        .copied()
        .collect()
}

/// Divide the left sidebar into header (project name), sections, and footer.
pub fn split_left(area: Rect) -> (Rect, Rect, Rect) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),           // header: project name
            Constraint::Min(4),              // main: file tree OR history list
            Constraint::Length(3),           // footer: tab bar [Files|History]
        ])
        .split(area);

    (chunks[0], chunks[1], chunks[2])
}
