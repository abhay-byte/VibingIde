# VibingIDE - Current Product Snapshot

Last reviewed: 2026-03-27

This document is intentionally grounded in the current codebase. Planned features are called out explicitly instead of being described as already available.

## 1. Product definition

VibingIDE is a Rust desktop app for running CLI coding agents side by side inside PTY-backed panels. The current implementation is agent-first, but it is not a full IDE yet: there is no built-in editor, and several navigation/history features are still in progress.

## 2. What works today

### Workspace shell

- Launches as a native desktop window using `egui`/`eframe`
- Opens the current directory by default
- Accepts a project path either positionally or with `--project`
- Accepts an optional initial agent command with `--cmd`
- Uses a custom toolbar with window controls, Help, and New Panel actions

### Agent panels

- Multiple panels can run at the same time
- Each panel owns its own PTY-backed child process
- Output streams into a scrollable buffer
- Basic ANSI colors and text styles are rendered
- Each panel has its own input box and Send button
- Panels can be focused by clicking
- Panels can be closed from the header button

### Sidebar

- Files tab shows a startup snapshot of the project tree
- History tab shows existing session metadata loaded from `.vibingide/index.json`

### Runtime and safety

- Child processes are launched without implicit shell interpolation
- Child environment variables are filtered through an allowlist
- Project paths are canonicalized before use
- Logs are written to `~/.vibingide/debug.log`

## 3. Current limitations

- No built-in code editor
- No live file watching or automatic tree refresh
- No project switcher or recent-project picker
- No command palette
- No panel maximize/restore flow in the active GUI
- No divider dragging between panels
- History persistence is incomplete
  - The app generates session IDs
  - The app can read existing session metadata
  - The app does not yet write session events or update `index.json`
- Keyboard support is partial
  - `Ctrl+N` opens the new-panel dialog
  - `Ctrl+C` closes the app window
  - Most other documented shortcut names exist in config/schema only
- Command parsing is simple whitespace splitting, so quoted arguments with embedded spaces are not preserved

## 4. User experience today

The current layout is:

- Top toolbar for app-level actions
- Left sidebar with Files and History tabs
- Right workspace with one column per open agent panel

If no panels are open, the app shows an empty state with a button to start a new agent panel.

## 5. CLI surface

Current supported arguments:

```text
vibingide [project]
vibingide --project <dir>
vibingide --cmd <command>
vibingide --project <dir> --cmd <command>
vibingide --help
vibingide --version
```

Examples:

```bash
vibingide
vibingide ~/repos/my-project
vibingide --project ~/repos/my-project --cmd "codex"
```

## 6. Config surface

The app loads:

- `~/.vibingide/config.toml`
- `<project>/.vibingide/config.toml`

Important caveat:

- The current implementation does not fully merge these files field-by-field.
- If a project config exists, it effectively replaces the global config structure.
- Only part of the schema is actively used by the running GUI today.

## 7. What is planned next

Near-term work that matches the current architecture:

- Wire session persistence end to end
- Make the History tab live-update for new sessions
- Connect the intended keyboard shortcuts in the egui app
- Hook PTY resize to panel/window changes
- Improve file-tree interaction and refresh
- Improve command parsing for quoted arguments and paths with spaces

## 8. Explicitly out of scope for the current implementation

- Full editor replacement for VS Code/Cursor
- LSP and autocomplete
- Git UI
- Plugin system
- Cloud sync
