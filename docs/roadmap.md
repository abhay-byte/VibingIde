# VibingIDE — Development Roadmap

## Timeline Overview

```mermaid
gantt
    title VibingIDE v0.1 — Development Schedule
    dateFormat  YYYY-MM-DD
    axisFormat  Week %W

    section Milestone 0 — Scaffold
    Cargo workspace + deps       :m0a, 2024-03-25, 3d
    Two-panel TUI skeleton        :m0b, after m0a, 3d
    CI pipeline                   :m0c, after m0a, 4d

    section Milestone 1 — Single Panel
    PTY supervisor               :m1a, after m0b, 4d
    ANSI parser + output buffer  :m1b, after m1a, 3d
    Input bar widget             :m1c, after m1b, 2d
    Windows ConPTY integration   :m1d, after m1b, 3d

    section Milestone 2 — Multi-Panel
    Panel manager CRUD           :m2a, after m1c, 3d
    Keyboard navigation          :m2b, after m2a, 2d
    Panel resize + maximize      :m2c, after m2b, 3d

    section Milestone 3 — History
    NDJSON session store         :m3a, after m2a, 3d
    History browser widget       :m3b, after m3a, 4d
    Replay / read-only view      :m3c, after m3b, 3d

    section Milestone 4 — File Explorer
    File tree widget             :m4a, after m3b, 4d
    Project open / recents       :m4b, after m4a, 3d

    section Milestone 5 — Polish
    Config system                :m5a, after m4a, 3d
    Command palette              :m5b, after m5a, 3d
    Theme + performance          :m5c, after m5b, 4d

    section Milestone 6 — Release
    Binary build matrix          :m6a, after m5c, 3d
    Packages / install scripts   :m6b, after m6a, 3d
```

---

## Milestone Dependency Graph

```mermaid
graph LR
    M0[M0\nScaffold] --> M1[M1\nSingle Panel]
    M1 --> M2[M2\nMulti-Panel]
    M2 --> M3[M3\nHistory]
    M2 --> M4[M4\nFile Explorer]
    M3 --> M5[M5\nPolish]
    M4 --> M5
    M5 --> M6[M6\nRelease v0.1]
```

---

## 🟢 Milestone 0 — Scaffolding

**Goal**: Working Rust project that compiles; minimal TUI skeleton visible.

```mermaid
flowchart TD
    A[cargo new vibingide] --> B[Add dependencies\nratatui crossterm tokio\nserde anyhow tracing]
    B --> C[Two-panel layout\nleft sidebar + right area]
    C --> D[Event loop\n60fps draw + Ctrl+C quit]
    D --> E[CI: GitHub Actions\nmulti-platform matrix]
    E --> DONE{✅ Render two-panel TUI\nResize reflows}
```

### Tasks
- [ ] Initialize Cargo workspace
- [ ] Add dependencies
- [ ] Basic two-panel TUI layout
- [ ] Main event loop: 60fps render, `Ctrl+C` to quit
- [ ] CI pipeline: Linux / Windows / macOS
- [ ] Logging to `~/.vibingide/debug.log` via `tracing`

---

## 🟡 Milestone 1 — Single Agent Panel

**Goal**: Spawn one CLI tool, see its ANSI output, send it input.

```mermaid
flowchart TD
    A[PTY Supervisor\nspawn child process] --> B[Async PTY reader\nTokio task]
    B --> C[vte ANSI parser\nVec StyledLine]
    C --> D[Ring buffer\nVecDeque 10k lines]
    D --> E[Output viewport widget\nscrollable 60fps]
    E --> F[Input bar\nstdin writer]
    F --> DONE{✅ vibingide --cmd claude\nworks end-to-end}
```

### Tasks
- [ ] PTY supervisor: spawn arbitrary command securely (no shell=true)
- [ ] ANSI parser pipeline → styled ring buffer
- [ ] Output viewport widget (scrollable, ANSI-colored)
- [ ] Input bar → stdin
- [ ] Panel resize → `pty.resize()`
- [ ] Windows ConPTY tested

---

## 🟠 Milestone 2 — Multi-Panel Support

**Goal**: Multiple independent agent panels, keyboard navigation.

### Tasks
- [ ] Panel manager: create, focus, close panels
- [ ] Vertical stacking layout with draggable dividers
- [ ] Keybindings: new, navigate, close, maximize
- [ ] Visual focus indicator (border color)
- [ ] Panel rename

---

## 🔵 Milestone 3 — Conversation History

**Goal**: Every session recorded and browsable.

```mermaid
flowchart LR
    A[Panel spawned] -->|SessionManager| B[New ULID session\n.ndjson created]
    B --> C[Events appended\non every I/O]
    C --> D[index.json updated\non start/end]
    D --> E[History List widget\nleft panel]
    E -->|click| F[Read-only\nreplay view]
```

### Tasks
- [ ] NDJSON session store with strict serde deserialization
- [ ] `index.json` maintenance
- [ ] History List widget in left panel
- [ ] Read-only replay viewer
- [ ] Auto-cleanup: archive sessions older than N days

---

## 🟣 Milestone 4 — File Explorer + Project Management

**Goal**: Left panel file tree, project open/switch.

### Tasks
- [ ] File tree widget (recursive, respects `.gitignore`)
- [ ] Keyboard navigation
- [ ] Copy path to clipboard
- [ ] `--project <path>` CLI flag + startup picker
- [ ] Recent projects list
- [ ] Project switch

---

## ⚪ Milestone 5 — Polish + Config

**Goal**: Production-ready UX and configuration system.

### Tasks
- [ ] Global + project config parsing
- [ ] Keybinding customization
- [ ] Command palette (`Ctrl+P`)
- [ ] Keybinding help overlay (`?`)
- [ ] Theme: dark / light / custom
- [ ] Toast notifications
- [ ] Performance: < 60 MB RAM, < 250 ms startup

---

## 🚀 Milestone 6 — Release v0.1

### Tasks
- [ ] GitHub Actions release matrix → binary artifacts
- [ ] LTO + strip + `panic = abort` optimization
- [ ] Install scripts (Linux/macOS curl-pipe, Windows winget)
- [ ] README with demo GIF
- [ ] CHANGELOG

---

## 🔮 Post v0.1 Backlog

```mermaid
graph LR
    V1[v0.1\nTUI Core] --> G[Native GUI shell\nwinit + wgpu]
    V1 --> ED[File Editor pane\ntree-sitter highlighting]
    V1 --> LS[LSP integration]
    V1 --> GIT[Git status overlay]
    V1 --> PL[Plugin system\nWASM extensions]
    V1 --> SSH[SSH project mode]
    V1 --> SRCH[Full-text search\nin history]
```
