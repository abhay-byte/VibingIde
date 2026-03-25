# VibingIDE — Technical Architecture

## 1. Technology Stack

| Layer | Technology | Rationale |
|---|---|---|
| Language | **Rust (stable)** | Memory safety, zero-cost abstractions, no GC pauses |
| UI Rendering | **[Ratatui](https://github.com/ratatui-org/ratatui)** | TUI framework; battle-tested, GPU-free, cross-platform |
| Terminal backend | **Crossterm** | Cross-platform terminal I/O |
| PTY (Unix) | **`portable-pty`** crate | Spawns processes in a pseudo-terminal |
| PTY (Windows) | **ConPTY** via `portable-pty` | Windows Console Pseudoconsole API |
| ANSI parsing | **`vte`** crate | VT100/ANSI escape sequence state machine |
| Async runtime | **Tokio** | I/O multiplexing across multiple PTY streams |
| Serialization | **Serde + serde_json** | Session history persistence (NDJSON) |
| Config | **`toml`** crate | User config file parsing |
| File watching | **`notify`** crate | Watch project directory for file tree changes |

> **Why TUI over GUI?** A terminal UI keeps the binary < 25 MB, starts instantly, and works over SSH — a common agent-use-case.

---

## 2. High-Level Architecture

```mermaid
graph TB
    subgraph VibingIDE["VibingIDE Process"]
        subgraph TUI["TUI Layer (Ratatui)"]
            LP[Left Panel]
            RP[Right Panels]
        end

        subgraph Core["Core Engine"]
            PM[Panel Manager]
            SM[Session Manager]
            PRJ[Project]
        end

        subgraph PTY["PTY Layer"]
            SUP1[Supervisor 1\nAgent Panel 1]
            SUP2[Supervisor 2\nAgent Panel 2]
            SUPN[Supervisor N\n...]
        end

        subgraph History["History Store"]
            NDJSON[(NDJSON Files\n.vibingide/sessions/)]
            IDX[(index.json)]
        end

        subgraph Input["Input Handler"]
            KBD[Keyboard Events\ncrossterm]
        end
    end

    subgraph Agents["External CLI Agents"]
        C1[claude]
        C2[opencode]
        CN[any CLI...]
    end

    KBD -->|AppAction| PM
    TUI -->|render| Core
    PM --> SUP1 & SUP2 & SUPN
    SM --> NDJSON & IDX
    SUP1 -.->|PTY| C1
    SUP2 -.->|PTY| C2
    SUPN -.->|PTY| CN
    SUP1 & SUP2 & SUPN -->|AppEvent mpsc| Core
    Core --> TUI
    SM --> LP
```

---

## 3. Module Structure

```mermaid
graph LR
    main["main.rs\nentry point"]
    app["app.rs\nAppState + event loop"]
    config["config.rs\nconfig loading"]

    subgraph ui["ui/"]
        layout["layout.rs"]
        left["left_panel.rs"]
        right["right_panel.rs"]
        agent_ui["agent_panel.rs"]
        history_ui["history_list.rs"]
        filetree_ui["file_tree.rs"]
        keybind_ui["keybind_overlay.rs"]
    end

    subgraph engine["engine/"]
        panel_mgr["panel_manager.rs"]
        session_mgr["session_manager.rs"]
        project["project.rs"]
    end

    subgraph pty_mod["pty/"]
        supervisor["supervisor.rs"]
        reader["reader.rs"]
        ansi["ansi.rs"]
    end

    subgraph history_mod["history/"]
        store["store.rs"]
        event["event.rs"]
    end

    subgraph input_mod["input/"]
        handler["handler.rs"]
        keybinds["keybinds.rs"]
    end

    main --> app
    app --> config & ui & engine & input_mod
    engine --> pty_mod & history_mod & project
    pty_mod --> ansi
```

---

## 4. Core Data Flow

### 4.1 Startup Sequence

```mermaid
sequenceDiagram
    participant M as main()
    participant C as Config
    participant P as Project
    participant S as SessionManager
    participant A as App
    participant T as TUI Loop

    M->>C: Config::load()
    C-->>M: AppConfig
    M->>P: Project::open(path)
    P-->>M: Project (file tree)
    M->>S: SessionManager::load(project)
    S-->>M: Vec<SessionMeta>
    M->>A: App::new(config, project, sessions)
    A->>T: run_event_loop()
    T-->>T: draw @ 60fps + poll events
```

### 4.2 Adding an Agent Panel

```mermaid
sequenceDiagram
    participant U as User
    participant IH as InputHandler
    participant PM as PanelManager
    participant SUP as Supervisor
    participant SM as SessionManager
    participant RT as Tokio Task
    participant BUF as OutputBuffer

    U->>IH: Ctrl+Shift+N
    IH->>PM: AppAction::NewPanel(cmd)
    PM->>SUP: Supervisor::spawn(cmd)
    Note over SUP: fork child + open PTY<br/>sanitize cmd (no shell injection)
    SUP-->>PM: PanelId, PID
    PM->>SM: new_session(panel_id)
    SM-->>PM: SessionId (ULID)
    PM->>RT: tokio::spawn(reader_loop)
    loop PTY Output
        RT->>BUF: AnsiParser::feed(bytes) → StyledLines
        RT->>SM: HistoryStore::append(AgentOutput)
        RT->>PM: AppEvent::PtyOutput (mpsc)
    end
```

### 4.3 User Sends Input

```mermaid
sequenceDiagram
    participant U as User
    participant IH as InputHandler
    participant HS as HistoryStore
    participant SUP as Supervisor

    U->>IH: Type text → Enter
    IH->>HS: append(UserInput { text })
    IH->>SUP: write_stdin(text + "\n")
    Note over SUP: write() to PTY master fd
```

### 4.4 TUI Render Loop

```mermaid
flowchart TD
    A[Poll AppEvent mpsc] --> B{Event type?}
    B -->|PtyOutput| C[Update panel output buffer]
    B -->|KeyEvent| D[InputHandler → AppAction]
    B -->|Resize| E[Recalculate layout\n+ pty.resize]
    B -->|Tick| F[No-op, trigger redraw]
    B -->|PtyExited| G[Mark panel Crashed/Exited\nFinalize session]
    C & D & E & F & G --> H[terminal.draw]
    H --> I[render_left_panel\nfile tree + history]
    H --> J[render_right_panels\none widget per panel]
    I & J --> A
```

---

## 5. PTY Supervision

```mermaid
graph LR
    subgraph Supervisor
        MASTER[PTY Master\nReadablePty]
        SLAVE[PTY Slave]
        CHILD[Child Process\nCLI Agent]
        READER[Async Reader\nTokio Task]
        ANSI[vte Parser\nStyledLines]
    end

    EVENT[AppEvent::PtyOutput\nmpsc channel]

    SLAVE --> CHILD
    CHILD -->|stdout/stderr| SLAVE
    SLAVE --> MASTER
    MASTER --> READER
    READER --> ANSI
    ANSI --> EVENT

    NOTE["Windows: ConPTY via\nportable-pty native_pty_system()\nUnix: openpty(2) + fork/exec"]
```

**Resize handling:**
1. `Event::Resize(cols, rows)` from crossterm
2. App recalculates panel dimensions
3. `Supervisor::resize(PtySize { rows, cols })` called per panel
4. SIGWINCH delivered to child automatically (Unix) / ConPTY notified (Windows)

---

## 6. Security Model

```mermaid
graph TD
    subgraph Threats["Threat Surface"]
        T1[Command Injection via\nagent cmd input]
        T2[Path Traversal in\nproject/config paths]
        T3[Malicious NDJSON\nin history files]
        T4[Unbounded memory from\nrogue PTY output]
        T5[Env var leakage\nto child processes]
    end

    subgraph Mitigations["Mitigations"]
        M1[execvp-style spawn — no shell\nargs as Vec not string]
        M2[PathBuf::canonicalize +\nprefix check vs project root]
        M3[Serde strict deserialization\nskip unknown fields]
        M4[Ring buffer cap\nmax 10k lines per panel]
        M5[Explicit env allowlist\nfor child process]
    end

    T1 --> M1
    T2 --> M2
    T3 --> M3
    T4 --> M4
    T5 --> M5
```

---

## 7. State Management

All mutable state lives in a single `AppState` struct. Background reader tasks communicate via `mpsc` channels — **no shared `Arc<Mutex<>>`**, eliminating lock contention.

```mermaid
graph LR
    subgraph Tasks["Background Tokio Tasks"]
        R1[Reader Task\nPanel 1]
        R2[Reader Task\nPanel 2]
        KT[Keyboard Task]
        FW[File Watcher Task]
    end

    CH[mpsc::channel\nAppEvent]
    MAIN[Main Loop\nAppState ownership]

    R1 & R2 & KT & FW -->|send| CH
    CH -->|recv| MAIN
    MAIN -->|mutate| MAIN
    MAIN -->|render| TUI[Ratatui Terminal]
```

---

## 8. History Storage

Session files live at `<project-root>/.vibingide/sessions/<ULID>.ndjson`.

```mermaid
graph LR
    subgraph DiskLayout[".vibingide/"]
        IDX[index.json\nsession listing]
        subgraph Sessions["sessions/"]
            S1["01HT8X3B2...ndjson"]
            S2["01HT9Y4C3...ndjson"]
        end
        CFG[config.toml\nproject overrides]
    end

    subgraph NDJSON["NDJSON Line Events"]
        E1["session_start"]
        E2["user_input"]
        E3["agent_output"]
        E4["session_end"]
    end

    S1 --> E1 & E2 & E3 & E4
```

---

## 9. Build & Release

```toml
[profile.release]
opt-level     = 3
lto           = "fat"
codegen-units = 1
strip         = true
panic         = "abort"
```

**Cross-compilation targets:**

```mermaid
graph LR
    RUST[Rust Source] --> LIN[x86_64-unknown-linux-gnu]
    RUST --> WIN[x86_64-pc-windows-msvc]
    RUST --> MAC[x86_64-apple-darwin]
    RUST --> ARM[aarch64-apple-darwin]
```

CI: GitHub Actions matrix build + `cargo-nextest`.
