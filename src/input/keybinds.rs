//! Keybinding definitions and help text.

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

/// Parsed key combination.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KeyBind {
    pub code:      KeyCode,
    pub modifiers: KeyModifiers,
}

impl KeyBind {
    pub fn matches(&self, event: &KeyEvent) -> bool {
        event.code == self.code && event.modifiers == self.modifiers
    }
}

/// All application keybinds in one place.
pub struct Keybinds {
    pub new_panel:       KeyBind,
    pub next_panel:      KeyBind,
    pub prev_panel:      KeyBind,
    pub focus_input:     KeyBind,
    pub focus_tree:      KeyBind,
    pub focus_history:   KeyBind,
    pub maximize_panel:  KeyBind,
    pub close_panel:     KeyBind,
    pub quit:            KeyBind,
    pub help_toggle:     KeyBind,
    pub scroll_up:       KeyBind,
    pub scroll_down:     KeyBind,
    pub scroll_up_page:  KeyBind,
    pub scroll_down_page:KeyBind,
}

impl Default for Keybinds {
    fn default() -> Self {
        use KeyCode::*;
        use KeyModifiers as KM;

        Self {
            new_panel:        KeyBind { code: Char('n'), modifiers: KM::CONTROL | KM::SHIFT },
            next_panel:       KeyBind { code: Char(']'), modifiers: KM::CONTROL },
            prev_panel:       KeyBind { code: Char('['), modifiers: KM::CONTROL },
            focus_input:      KeyBind { code: Char('i'), modifiers: KM::CONTROL },
            focus_tree:       KeyBind { code: Char('e'), modifiers: KM::CONTROL },
            focus_history:    KeyBind { code: Char('h'), modifiers: KM::CONTROL },
            maximize_panel:   KeyBind { code: Char('m'), modifiers: KM::CONTROL },
            close_panel:      KeyBind { code: Char('w'), modifiers: KM::CONTROL },
            quit:             KeyBind { code: Char('c'), modifiers: KM::CONTROL },
            help_toggle:      KeyBind { code: Char('?'), modifiers: KM::NONE },
            scroll_up:        KeyBind { code: Char('u'), modifiers: KM::CONTROL },
            scroll_down:      KeyBind { code: Char('d'), modifiers: KM::CONTROL },
            scroll_up_page:   KeyBind { code: PageUp,    modifiers: KM::NONE },
            scroll_down_page: KeyBind { code: PageDown,  modifiers: KM::NONE },
        }
    }
}

/// Human-readable help entries shown in the keybind overlay.
pub fn help_entries() -> Vec<(&'static str, &'static str)> {
    vec![
        ("Ctrl+Shift+N", "New agent panel"),
        ("Ctrl+]",       "Next panel"),
        ("Ctrl+[",       "Previous panel"),
        ("Ctrl+I",       "Focus input bar"),
        ("Ctrl+E",       "Focus file tree"),
        ("Ctrl+H",       "Focus history"),
        ("Ctrl+M",       "Maximize/restore panel"),
        ("Ctrl+W",       "Close panel"),
        ("Ctrl+U / ↑",   "Scroll up"),
        ("Ctrl+D / ↓",   "Scroll down"),
        ("PgUp / PgDn",  "Scroll page"),
        ("?",            "Toggle this help"),
        ("Ctrl+C",       "Quit"),
    ]
}
