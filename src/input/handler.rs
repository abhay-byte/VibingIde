//! Maps crossterm `KeyEvent`s to `AppAction` variants.

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use super::keybinds::Keybinds;

/// All actions the application can perform based on user input.
#[derive(Debug, Clone, PartialEq)]
pub enum AppAction {
    Quit,
    NewPanel,
    NextPanel,
    PrevPanel,
    ClosePanel,
    MaximizePanel,
    FocusInput,
    FocusTree,
    FocusHistory,
    ToggleHelp,
    ScrollUp(usize),
    ScrollDown(usize),
    /// Character typed into the input bar.
    InputChar(char),
    /// Backspace in input bar.
    InputBackspace,
    /// Submit current input bar content to the agent.
    InputSubmit,
    /// Arrow up in input bar — cycle history (future).
    InputHistoryPrev,
    /// Arrow down in input bar — cycle history (future).
    InputHistoryNext,
    /// Unhandled/unknown key.
    Noop,
}

pub struct InputHandler {
    keybinds: Keybinds,
}

impl InputHandler {
    pub fn new(keybinds: Keybinds) -> Self {
        Self { keybinds }
    }

    /// Translate a raw crossterm `KeyEvent` into an `AppAction`.
    pub fn handle(&self, event: KeyEvent, in_input_bar: bool) -> AppAction {
        // ── Global bindings (always active) ──
        if self.keybinds.quit.matches(&event) {
            return AppAction::Quit;
        }
        if self.keybinds.help_toggle.matches(&event) && !in_input_bar {
            return AppAction::ToggleHelp;
        }

        // ── Input-bar mode ───────────────────────────────────────────────────
        if in_input_bar {
            return match event.code {
                KeyCode::Char(c) => {
                    // Ctrl+C is handled globally above; block other ctrl combos.
                    if event.modifiers.contains(KeyModifiers::CONTROL) {
                        AppAction::Noop
                    } else {
                        AppAction::InputChar(c)
                    }
                }
                KeyCode::Backspace => AppAction::InputBackspace,
                KeyCode::Enter     => AppAction::InputSubmit,
                KeyCode::Up        => AppAction::InputHistoryPrev,
                KeyCode::Down      => AppAction::InputHistoryNext,
                KeyCode::Esc       => AppAction::FocusTree, // Escape leaves input bar
                _                  => AppAction::Noop,
            };
        }

        // ── Navigation mode (input bar not focused) ──────────────────────────
        if self.keybinds.new_panel.matches(&event)      { return AppAction::NewPanel; }
        if self.keybinds.next_panel.matches(&event)     { return AppAction::NextPanel; }
        if self.keybinds.prev_panel.matches(&event)     { return AppAction::PrevPanel; }
        if self.keybinds.focus_input.matches(&event)    { return AppAction::FocusInput; }
        if self.keybinds.focus_tree.matches(&event)     { return AppAction::FocusTree; }
        if self.keybinds.focus_history.matches(&event)  { return AppAction::FocusHistory; }
        if self.keybinds.maximize_panel.matches(&event) { return AppAction::MaximizePanel; }
        if self.keybinds.close_panel.matches(&event)    { return AppAction::ClosePanel; }
        if self.keybinds.scroll_up.matches(&event)
            || event.code == KeyCode::Up
        {
            return AppAction::ScrollUp(3);
        }
        if self.keybinds.scroll_down.matches(&event)
            || event.code == KeyCode::Down
        {
            return AppAction::ScrollDown(3);
        }
        if self.keybinds.scroll_up_page.matches(&event)   { return AppAction::ScrollUp(20); }
        if self.keybinds.scroll_down_page.matches(&event) { return AppAction::ScrollDown(20); }

        AppAction::Noop
    }
}
