//! Click-zone hit-testing for mouse support.
//!
//! After every frame draw, the app builds a registry of `(ClickZone, Rect)` pairs.
//! Mouse events are mapped to actions by searching this registry.

use ratatui::layout::Rect;

use crate::pty::supervisor::PanelId;

/// Every interactive region in the UI.
#[derive(Debug, Clone, PartialEq)]
pub enum ClickZone {
    // Toolbar
    ToolbarNewPanel,
    ToolbarHelp,
    ToolbarQuit,

    // Left panel
    LeftTabFiles,
    LeftTabHistory,

    // Agent panel regions (keyed by panel id)
    PanelFocus(PanelId),
    PanelClose(PanelId),
    PanelScrollUp(PanelId),
    PanelScrollDown(PanelId),
    PanelInputBar(PanelId),
    PanelSendButton(PanelId),

    // Empty-state action
    EmptyNewPanel,
}

/// A registered interactive zone with its screen rect.
#[derive(Debug, Clone)]
pub struct Zone {
    pub kind: ClickZone,
    pub rect: Rect,
}

impl Zone {
    pub fn new(kind: ClickZone, rect: Rect) -> Self {
        Self { kind, rect }
    }
}

/// Find the topmost zone that contains the point `(x, y)`.
pub fn hit_test(zones: &[Zone], x: u16, y: u16) -> Option<&ClickZone> {
    // Iterate in reverse so later-registered (foreground) zones win.
    zones.iter().rev().find_map(|z| {
        if rect_contains(z.rect, x, y) {
            Some(&z.kind)
        } else {
            None
        }
    })
}

#[inline]
fn rect_contains(r: Rect, x: u16, y: u16) -> bool {
    x >= r.x && x < r.x + r.width && y >= r.y && y < r.y + r.height
}
