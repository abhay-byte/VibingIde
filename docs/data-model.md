# VibingIDE — Data Model

## 1. Entity Relationships

```mermaid
erDiagram
    PROJECT ||--o{ SESSION_META : "has many"
    PROJECT ||--|| PROJECT_CONFIG : "has one"
    SESSION_META ||--|| AGENT_PANEL : "linked to"
    AGENT_PANEL ||--o{ HISTORY_EVENT : "records"
    SESSION_META }o--|| NDJSON_FILE : "stored in"

    PROJECT {
        PathBuf root_path
        String  name
        FileTree file_tree
        PathBuf sessions_dir
    }

    AGENT_PANEL {
        u32     id
        String  label
        String  command
        String  session_id
        Enum    status
        usize   scroll_pos
        String  input_buf
        u32     pid
        DateTime started_at
    }

    SESSION_META {
        String   session_id
        String   panel_label
        String   agent_cmd
        DateTime started_at
        DateTime ended_at
        Enum     status
        String   first_input
        usize    message_count
    }

    HISTORY_EVENT {
        i64    ts
        Enum   kind
        String text
        i32    exit_code
    }
```

---

## 2. Core Rust Structs

### 2.1 `Project`

```rust
struct Project {
    root_path:    PathBuf,     // Absolute path — canonicalized on open
    name:         String,      // Display name (directory basename)
    file_tree:    FileTree,    // Cached listing, refreshed on change
    sessions_dir: PathBuf,     // <root>/.vibingide/sessions/
    config:       ProjectConfig,
}
```

### 2.2 `AgentPanel`

```rust
struct AgentPanel {
    id:          PanelId,             // u32 counter
    label:       String,              // "Claude Code #1"
    command:     String,              // "claude"
    args:        Vec<String>,         // CLI args — execvp style, no shell
    status:      PanelStatus,
    session_id:  SessionId,           // ULID → links to NDJSON file
    scroll_pos:  usize,
    output_buf:  VecDeque<StyledLine>,// Ring buffer, max 10k lines
    input_buf:   String,
    pid:         Option<u32>,
    started_at:  DateTime<Utc>,
    ended_at:    Option<DateTime<Utc>>,
}

enum PanelStatus {
    Starting,
    Running,
    Exited { code: i32 },
    Crashed { signal: Option<i32> },
}
```

### 2.3 `HistoryEvent` (NDJSON line)

```rust
#[derive(Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
enum HistoryEvent {
    SessionStart { ts: i64, agent_cmd: String, label: String, cwd: String },
    UserInput    { ts: i64, text: String },
    AgentOutput  { ts: i64, text: String },
    SessionEnd   { ts: i64, exit_code: Option<i32>, signal: Option<i32> },
}
```

---

## 3. AppState & Event Model

```mermaid
classDiagram
    class AppState {
        +Project project
        +Vec~AgentPanel~ panels
        +PanelId focused_panel
        +FocusZone focus_zone
        +Vec~SessionMeta~ sessions
        +bool show_keybind_help
        +f32 layout_split
    }

    class FocusZone {
        <<enumeration>>
        LeftTree
        LeftHistory
        RightPanel
        InputBar
    }

    class AppEvent {
        <<enumeration>>
        PtyOutput
        PtyExited
        KeyEvent
        Resize
        FileTreeChanged
        Tick
    }

    class PanelStatus {
        <<enumeration>>
        Starting
        Running
        Exited
        Crashed
    }

    AppState --> FocusZone
    AppState --> AgentPanel
    AgentPanel --> PanelStatus
```

### Event flow (no shared mutex)

```mermaid
sequenceDiagram
    participant BG as Background Tasks\n(PTY readers, keyboard, fs watcher)
    participant CH as mpsc Channel\nAppEvent
    participant ML as Main Loop\n(owns AppState)
    participant TUI as Ratatui Terminal

    BG->>CH: send(AppEvent)
    CH->>ML: recv()
    ML->>ML: mutate AppState
    ML->>TUI: terminal.draw(render_fn)
```

---

## 4. File Layout on Disk

```mermaid
graph TD
    ROOT["&lt;project-root&gt;/"]
    VID[".vibingide/"]
    CFG["config.toml\nproject overrides"]
    IDX["index.json\nsession listing"]
    SESS["sessions/"]
    S1["01HT8X3B...ndjson"]
    S2["01HT9Y4C...ndjson"]

    HOME["~/.vibingide/"]
    GCFG["config.toml\nglobal defaults"]
    REC["recents.json\nrecent projects"]

    ROOT --> VID
    VID --> CFG & IDX & SESS
    SESS --> S1 & S2

    HOME --> GCFG & REC
```

### `index.json` format

```json
{
  "version": 1,
  "sessions": [
    {
      "session_id": "01HT8X3B2HYZK6E8VBXPQ5NRCS",
      "label": "Claude Code #1",
      "agent_cmd": "claude",
      "started_at": "2024-03-24T10:00:00Z",
      "ended_at":   "2024-03-24T10:45:00Z",
      "status": "closed",
      "first_input": "refactor the auth module",
      "message_count": 42
    }
  ]
}
```

---

## 5. Output Buffer Model

```mermaid
graph LR
    PTY[PTY Master\nbytes] --> VTE[vte\nparser]
    VTE --> SL["Vec&lt;StyledCell&gt;\nper line"]
    SL --> RB["VecDeque&lt;StyledLine&gt;\nring buffer\nmax 10k lines"]
    RB --> RAT[Ratatui\nSpans → render]
    RB -->|"pop_front when full"| DEL[discarded]
```

```rust
struct StyledLine  { cells: Vec<StyledCell> }
struct StyledCell  { ch: char, fg: Color, bg: Color, modifiers: Modifiers }
```

---

## 6. Configuration Schema

```mermaid
graph TD
    GC["~/.vibingide/config.toml\nglobal defaults"]
    PC["&lt;root&gt;/.vibingide/config.toml\nproject overrides"]
    MC["Merged Config\n(project wins)"]

    GC --> MC
    PC --> MC
    MC --> APP[AppConfig used at runtime]
```

```toml
[ui]
theme                   = "dark"
left_panel_width_pct    = 25
output_buffer_lines     = 10000
scroll_speed            = 3
show_panel_borders      = true

[keybinds]
new_panel    = "ctrl+shift+n"
next_panel   = "ctrl+]"
prev_panel   = "ctrl+["
focus_input  = "ctrl+i"
focus_tree   = "ctrl+e"
focus_history = "ctrl+h"
maximize_panel = "ctrl+m"
close_panel  = "ctrl+w"

[history]
max_sessions_per_project = 500
auto_archive_after_days  = 30
store_raw_ansi           = false

[security]
child_env_allowlist = ["PATH", "HOME", "TERM", "LANG"]  # explicit passthrough
```
