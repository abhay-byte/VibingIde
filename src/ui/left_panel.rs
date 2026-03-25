//! Left panel widget — file tree and session history list.

use ratatui::{
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Paragraph},
    Frame,
};

use crate::engine::project::FileNode;
use crate::history::store::{SessionMeta, SessionStatus};

/// Which sub-view the left panel is showing.
#[derive(Debug, Clone, PartialEq)]
pub enum LeftPanelView {
    FileTree,
    History,
}

pub struct LeftPanelState {
    pub view:              LeftPanelView,
    pub file_tree_cursor:  usize,
    pub history_cursor:    usize,
}

impl Default for LeftPanelState {
    fn default() -> Self {
        Self {
            view:             LeftPanelView::FileTree,
            file_tree_cursor: 0,
            history_cursor:   0,
        }
    }
}

/// Render the full left panel (header + content + tab footer).
pub fn render_left_panel(
    frame: &mut Frame,
    header_area: Rect,
    content_area: Rect,
    footer_area: Rect,
    project_name: &str,
    state: &LeftPanelState,
    file_nodes: &[FileNode],
    sessions:   &[SessionMeta],
    focused:    bool,
) {
    render_header(frame, header_area, project_name);
    render_tab_footer(frame, footer_area, &state.view);

    match state.view {
        LeftPanelView::FileTree => {
            render_file_tree(frame, content_area, file_nodes, state.file_tree_cursor, focused);
        }
        LeftPanelView::History => {
            render_history_list(frame, content_area, sessions, state.history_cursor, focused);
        }
    }
}

fn render_header(frame: &mut Frame, area: Rect, project_name: &str) {
    let title = format!(" 📁 {project_name} ");
    let para = Paragraph::new(title)
        .block(
            Block::default()
                .borders(Borders::BOTTOM)
                .border_style(Style::default().fg(Color::DarkGray)),
        )
        .style(Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD));
    frame.render_widget(para, area);
}

fn render_tab_footer(frame: &mut Frame, area: Rect, view: &LeftPanelView) {
    let files_style = if *view == LeftPanelView::FileTree {
        Style::default().fg(Color::Black).bg(Color::Cyan).add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(Color::DarkGray)
    };
    let hist_style = if *view == LeftPanelView::History {
        Style::default().fg(Color::Black).bg(Color::Cyan).add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(Color::DarkGray)
    };

    let line = Line::from(vec![
        Span::raw(" "),
        Span::styled(" Files ", files_style),
        Span::raw(" "),
        Span::styled(" History ", hist_style),
        Span::raw(" "),
    ]);
    let para = Paragraph::new(line).block(
        Block::default()
            .borders(Borders::TOP)
            .border_style(Style::default().fg(Color::DarkGray)),
    );
    frame.render_widget(para, area);
}

fn render_file_tree(
    frame: &mut Frame,
    area: Rect,
    nodes: &[FileNode],
    cursor: usize,
    focused: bool,
) {
    let border_style = if focused {
        Style::default().fg(Color::Cyan)
    } else {
        Style::default().fg(Color::DarkGray)
    };

    let mut items = Vec::new();
    flatten_tree(nodes, 0, &mut items);

    let list_items: Vec<ListItem> = items
        .iter()
        .enumerate()
        .map(|(i, (indent, node))| {
            let prefix = "  ".repeat(*indent);
            let icon = if node.is_dir() { "📂 " } else { "   " };
            let name = &node.name;
            let style = if i == cursor && focused {
                Style::default().fg(Color::Black).bg(Color::Cyan)
            } else if node.is_dir() {
                Style::default().fg(Color::Yellow)
            } else {
                Style::default().fg(Color::White)
            };
            let line = Line::from(vec![
                Span::raw(format!("{prefix}{icon}")),
                Span::styled(name.as_str(), style),
            ]);
            ListItem::new(line)
        })
        .collect();

    let list = List::new(list_items).block(
        Block::default()
            .borders(Borders::NONE)
            .border_style(border_style),
    );
    frame.render_widget(list, area);
}

fn flatten_tree<'a>(nodes: &'a [FileNode], depth: usize, out: &mut Vec<(usize, &'a FileNode)>) {
    for node in nodes {
        out.push((depth, node));
        if node.is_dir() && !node.children.is_empty() {
            flatten_tree(&node.children, depth + 1, out);
        }
    }
}

fn render_history_list(
    frame: &mut Frame,
    area: Rect,
    sessions: &[SessionMeta],
    cursor: usize,
    focused: bool,
) {
    let items: Vec<ListItem> = sessions
        .iter()
        .enumerate()
        .rev() // newest first
        .map(|(i, s)| {
            let status_icon = match s.status {
                SessionStatus::Active  => Span::styled("● ", Style::default().fg(Color::Green)),
                SessionStatus::Closed  => Span::styled("○ ", Style::default().fg(Color::DarkGray)),
                SessionStatus::Crashed => Span::styled("✗ ", Style::default().fg(Color::Red)),
            };
            let label = Span::styled(
                s.label.as_str(),
                if i == cursor && focused {
                    Style::default().fg(Color::Black).bg(Color::Cyan)
                } else {
                    Style::default().fg(Color::White)
                },
            );
            let preview = s.first_input.as_deref().unwrap_or("…");
            let preview_span = Span::styled(
                format!(" {preview}"),
                Style::default().fg(Color::DarkGray),
            );
            ListItem::new(Line::from(vec![status_icon, label, preview_span]))
        })
        .collect();

    let list = List::new(items).block(Block::default().borders(Borders::NONE));
    frame.render_widget(list, area);
}
