# VibingIDE - Current Data Model

Last reviewed: 2026-03-27

This document describes the data structures and on-disk formats that exist in the current codebase. It also calls out where a schema exists but is not yet used end to end.

## 1. Core runtime structs

### Project

Source: [`src/engine/project.rs`](/c:/Users/abhay/repos/VibingIde/src/engine/project.rs)

```rust
struct Project {
    root: PathBuf,
    name: String,
    file_tree: Vec<FileNode>,
    vibide_dir: PathBuf,
}
```

Notes:

- `root` is expected to be canonicalized before `Project::open()`.
- `vibide_dir` is `<project>/.vibingide`.
- `file_tree` is a startup snapshot, not a live model kept in sync by watchers.

### FileNode

```rust
struct FileNode {
    name: String,
    path: PathBuf,
    kind: FileKind,
    children: Vec<FileNode>,
}

enum FileKind {
    File,
    Directory,
}
```

Scan limits:

- Maximum depth: `8`
- Maximum entries per directory: `500`
- Hidden directories are skipped
- `.vibingide` is skipped
- `.gitignore` support is simple filename matching only

### AgentPanel

Source: [`src/engine/panel_manager.rs`](/c:/Users/abhay/repos/VibingIde/src/engine/panel_manager.rs)

```rust
struct AgentPanel {
    id: u32,
    label: String,
    command: String,
    args: Vec<String>,
    status: PanelStatus,
    session_id: String,
    output_buf: VecDeque<StyledLine>,
    input_buf: String,
    scroll_pos: usize,
}
```

```rust
enum PanelStatus {
    Starting,
    Running { pid: u32 },
    Exited { code: i32 },
    Crashed { signal: Option<i32> },
}
```

Notes:

- `session_id` is assigned today even though persistence is not yet wired.
- `output_buf` is capped by `ui.output_buffer_lines`.
- `scroll_pos` is stored in the struct, but the current egui panel view mostly relies on `ScrollArea`.

### VibingApp

Source: [`src/app.rs`](/c:/Users/abhay/repos/VibingIde/src/app.rs)

```rust
struct VibingApp {
    project: Project,
    config: AppConfig,
    session_mgr: SessionManager,
    panel_mgr: PanelManager,
    ansi_parser: AnsiParser,
    pty_rx: UnboundedReceiver<PtyEvent>,
    _rt: Arc<Runtime>,
    sidebar_view: SidebarView,
    show_help: bool,
    show_new_panel_dialog: bool,
    new_panel_cmd: String,
    cmd_error: Option<String>,
}
```

This is the real top-level app state in the current GUI. Older docs that mention a Ratatui `AppState` no longer match the running application.

## 2. PTY and ANSI data

Source: [`src/pty/ansi.rs`](/c:/Users/abhay/repos/VibingIde/src/pty/ansi.rs)

```rust
struct StyledLine {
    cells: Vec<StyledCell>,
}

struct StyledCell {
    ch: char,
    style: CellStyle,
}

struct CellStyle {
    fg: Option<Color32>,
    bg: Option<Color32>,
    text: TextStyle,
}

struct TextStyle {
    bold: bool,
    italic: bool,
    underline: bool,
    dim: bool,
    strikethrough: bool,
}
```

Current ANSI support:

- Plain text
- Basic SGR styling
- Standard foreground/background colors
- Bright foreground colors

Not yet supported:

- Full terminal emulation
- 256-color and truecolor SGR handling
- Cursor-addressed screen state

## 3. Session metadata

Source: [`src/history/store.rs`](/c:/Users/abhay/repos/VibingIde/src/history/store.rs)

```rust
struct SessionMeta {
    session_id: String,
    label: String,
    agent_cmd: String,
    started_at: String,
    ended_at: Option<String>,
    status: SessionStatus,
    first_input: Option<String>,
    message_count: usize,
}
```

```rust
enum SessionStatus {
    Active,
    Closed,
    Crashed,
}
```

`SessionManager` currently does two things:

- load `index.json` into memory on startup
- generate new ULID-based session IDs

It does not yet:

- create `SessionMeta` for new panels
- save `index.json`
- connect history files to running panels

## 4. History event schema

Source: [`src/history/event.rs`](/c:/Users/abhay/repos/VibingIde/src/history/event.rs)

```rust
enum HistoryEvent {
    SessionStart { ts, agent_cmd, label, cwd },
    UserInput { ts, text },
    AgentOutput { ts, text },
    SessionEnd { ts, exit_code, signal },
}
```

This schema is defined and the NDJSON store can serialize it, but the live application does not currently emit these events.

## 5. On-disk layout

At runtime, opening a project ensures this structure exists:

```text
<project>/
  .vibingide/
    sessions/
```

Optional files the current code knows how to read:

```text
<project>/
  .vibingide/
    config.toml
    index.json
    sessions/
      <session-id>.ndjson
```

Global config path:

```text
~/.vibingide/config.toml
```

Log path:

```text
~/.vibingide/debug.log
```

## 6. Config schema

Source: [`src/config.rs`](/c:/Users/abhay/repos/VibingIde/src/config.rs)

```toml
[ui]
theme = "dark"
left_panel_width_pct = 25
output_buffer_lines = 10000
scroll_speed = 3
show_panel_borders = true
show_status_bar = true

[keybinds]
new_panel = "ctrl+shift+n"
next_panel = "ctrl+]"
prev_panel = "ctrl+["
focus_input = "ctrl+i"
focus_tree = "ctrl+e"
focus_history = "ctrl+h"
maximize_panel = "ctrl+m"
close_panel = "ctrl+w"
open_project = "ctrl+o"
command_palette = "ctrl+p"
scroll_up = "ctrl+u"
scroll_down = "ctrl+d"

[history]
max_sessions_per_project = 500
auto_archive_after_days = 30
store_raw_ansi = false

[security]
child_env_allowlist = ["PATH", "HOME", "TERM", "LANG"]
```

Important behavior notes:

- Unknown keys are rejected.
- `ui.left_panel_width_pct` and `ui.output_buffer_lines` are validated.
- Runtime currently uses `ui.output_buffer_lines` and `security.child_env_allowlist`.
- Most keybind and history settings are defined in schema but not yet applied by the egui UI.
- If a project config file exists, the current merge behavior effectively replaces the global config instead of merging field-by-field.
