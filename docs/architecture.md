# VibingIDE - Current Architecture

Last reviewed: 2026-03-27

This document describes the architecture that is actually in the repository today. Older drafts described a Ratatui/Crossterm terminal UI; the current implementation is a desktop GUI built with `egui` and `eframe`.

## 1. Stack

| Area | Current choice | Notes |
| --- | --- | --- |
| Language | Rust stable | Single binary application |
| GUI shell | `eframe` + `egui` | Native desktop window, custom chrome |
| Async/runtime | Tokio | Used for PTY reader tasks |
| Process isolation | `portable-pty` | PTY-backed child processes, ConPTY on Windows |
| ANSI parsing | `vte` | Basic color/style parsing into styled cells |
| Config | `serde` + `toml` | Global and per-project config loading |
| Persistence | `serde_json` | Session index and NDJSON helpers exist |
| Logging | `tracing` | Logs written to `~/.vibingide/debug.log` |

Notes:

- `notify` is present in `Cargo.toml` but is not wired into the running app yet.
- The `ui/` and `input/` folders still contain older TUI-oriented modules and helper code, but the active GUI is rendered directly from [`src/app.rs`](/c:/Users/abhay/repos/VibingIde/src/app.rs).

## 2. Runtime shape

Startup currently flows like this:

1. [`src/main.rs`](/c:/Users/abhay/repos/VibingIde/src/main.rs) parses `--project` and `--cmd`.
2. The project root is canonicalized and normalized with [`src/path_utils.rs`](/c:/Users/abhay/repos/VibingIde/src/path_utils.rs).
3. [`src/config.rs`](/c:/Users/abhay/repos/VibingIde/src/config.rs) loads config.
4. A Tokio runtime is created for PTY background work.
5. `eframe` launches the native window and constructs `VibingApp`.
6. [`src/app.rs`](/c:/Users/abhay/repos/VibingIde/src/app.rs) opens the project, loads any existing session index, and optionally spawns the initial agent panel.

## 3. Main components

### `main.rs`

- Sets up logging.
- Parses CLI arguments.
- Creates the Tokio runtime.
- Launches the `eframe` window.

### `app.rs`

`VibingApp` is the active application shell. It owns:

- `Project`
- `AppConfig`
- `SessionManager`
- `PanelManager`
- `AnsiParser`
- The PTY event receiver
- Transient GUI state such as dialogs and sidebar selection

Each `update()` call:

1. Drains PTY events from a Tokio `mpsc::UnboundedReceiver`.
2. Applies those events to panel state.
3. Renders the toolbar, sidebar, and agent panels.
4. Requests another repaint roughly 16 ms later for streaming output.

### `engine/project.rs`

- Scans the project tree at startup.
- Creates `.vibingide/` and `.vibingide/sessions/`.
- Skips `.vibingide`.
- Skips hidden directories.
- Applies a simple `.gitignore` filename filter.
- Caps traversal at depth 8 and 500 entries per directory.

This is a snapshot scan, not a live watcher.

### `engine/panel_manager.rs`

- Creates and tracks agent panels.
- Spawns PTY supervisors.
- Holds each panel's output ring buffer and input text.
- Tracks focused panel.
- Accepts PTY output and exit events.

The panel manager already has methods for focus cycling and PTY resize, but the current GUI only uses a subset of them.

### `pty/supervisor.rs`

- Spawns commands without implicit shell interpolation.
- Clears the child environment and re-adds an explicit allowlist.
- Reads PTY output on a blocking task and forwards bytes back to the GUI.
- Handles Windows command resolution for `.exe`, `.cmd`, `.bat`, and `.ps1`.

### `history/`

- [`src/history/event.rs`](/c:/Users/abhay/repos/VibingIde/src/history/event.rs) defines the NDJSON event schema.
- [`src/history/store.rs`](/c:/Users/abhay/repos/VibingIde/src/history/store.rs) can open session files, append events, read them back, and load/save `index.json`.
- [`src/engine/session_manager.rs`](/c:/Users/abhay/repos/VibingIde/src/engine/session_manager.rs) currently only loads existing metadata and generates new session IDs.

Important: history persistence helpers exist, but they are not yet connected to runtime panel creation, user input, PTY output, or panel shutdown.

## 4. Data flow

### Create panel

1. The user opens the "New Agent Panel" dialog or passes `--cmd`.
2. `app.rs` splits the command string with `split_whitespace()`.
3. `PanelManager::create_panel()` allocates a panel ID and label.
4. `Supervisor::spawn()` opens a PTY and launches the process.
5. The panel is inserted into in-memory state if spawning succeeds.

Current limitation:

- Command parsing is intentionally simple and does not preserve quoted arguments containing spaces.

### PTY output

1. `Supervisor` reads raw bytes from the PTY.
2. It sends `PtyEvent::Output` over the Tokio channel.
3. `VibingApp::drain_pty_events()` forwards bytes into `AnsiParser`.
4. Parsed `StyledLine`s are appended to the panel's ring buffer.
5. The GUI renders the buffered lines inside a scroll area.

### User input

1. Each panel has its own input field in the GUI.
2. Clicking "Send" or pressing Enter writes the input to PTY stdin.
3. The input field is cleared after successful submission.

Current limitation:

- Input is sent to the process, but not yet recorded into session history.

## 5. Security model

Current safeguards in code:

- Project root is canonicalized before use.
- File-tree traversal skips symlinks that resolve outside the project root.
- PTY processes are launched with explicit args, not shell-expanded command strings.
- Child environment variables are filtered through an allowlist.
- PTY dimensions are clamped before resize/spawn.
- Config parsing rejects unknown fields.
- Session-store helpers validate session IDs and restrict writes to the sessions directory.

## 6. Known gaps

- The docs previously claimed a complete TUI architecture. That is no longer true.
- History recording and replay are incomplete.
- Config loading is broader than config application. Today, runtime behavior only uses a few config values.
- Global/project config merge is coarse: if a project config exists, it currently replaces the global config structure instead of merging field-by-field.
- The GUI does not yet wire up the full keyboard/action system described by older docs and helper modules.
