# VibingIDE

> **Agent-First IDE. Blazing Fast. Built in Rust.**

VibingIDE is a super-lightweight, terminal-native IDE built around the paradigm of AI coding agents. Instead of bolting AI on top of a file editor, VibingIDE makes CLI agent sessions (Claude Code, Codex, OpenCode, Cline, Aider, etc.) the **primary interface** — with a project file tree and per-session conversation history as navigation support.

---

## ✨ Key Features

- **Multiple Agent Panels** — Run unlimited CLI AI tools side-by-side in independent PTY panels
- **Per-Panel Conversation History** — Every session is automatically saved; browse old sessions from the left sidebar
- **Tool-Agnostic** — Works with any CLI tool: `claude`, `opencode`, `codex`, `cline`, `aider`, or a custom shell script
- **Zero Bloat** — Pure Rust TUI; < 25 MB binary, < 60 MB RAM, < 250 ms startup
- **Cross-Platform** — Linux, macOS, Windows 10+ (via ConPTY)

---

## 🖼️ Layout

```
┌──────────────┬────────────────────────────────────────────────────────┐
│              │  ┌──── Agent Panel 1 (claude) ──────────────────────┐  │
│  📁 Files    │  │  > Working on auth module...                     │  │
│  (project    │  │  [INPUT BAR]                                     │  │
│   explorer)  │  └──────────────────────────────────────────────────┘  │
│              │                                                         │
│  💬 History  │  ┌──── Agent Panel 2 (opencode) ────────────────────┐  │
│  (per-panel  │  │  > Refactoring database layer...                 │  │
│   sessions)  │  │  [INPUT BAR]                                     │  │
│              │  └──────────────────────────────────────────────────┘  │
│  [+ Panel]   │  [+ Add Agent Panel]                                   │
└──────────────┴────────────────────────────────────────────────────────┘
```

---

## 📚 Documentation

| Document | Description |
|---|---|
| [specs.md](docs/specs.md) | Full product specification — features, layout, keybindings, performance targets |
| [architecture.md](docs/architecture.md) | Technical architecture — Rust stack, module tree, data flow, PTY supervision |
| [data-model.md](docs/data-model.md) | Data model — structs, NDJSON schema, config schema, disk layout |
| [roadmap.md](docs/roadmap.md) | Development roadmap — 6 milestones from scaffolding to v0.1 release |

---

## 🚀 Quick Start (planned)

```bash
# Install (Linux / macOS)
curl -fsSL https://get.vibingide.dev | sh

# Open a project directory
vibingide ~/repos/my-project

# Or specify an agent command to auto-launch
vibingide ~/repos/my-project --cmd "claude"
```

---

## ⌨️ Default Keybindings

| Action | Key |
|---|---|
| New agent panel | `Ctrl+Shift+N` |
| Next / Prev panel | `Ctrl+]` / `Ctrl+[` |
| Focus input bar | `Ctrl+I` |
| Focus file tree | `Ctrl+E` |
| Focus history | `Ctrl+H` |
| Maximize panel | `Ctrl+M` |
| Close panel | `Ctrl+W` |
| Open project | `Ctrl+O` |
| Command palette | `Ctrl+P` |
| Keybind help | `?` |

All keybindings are remappable in `~/.vibingide/config.toml`.

---

## 🛠️ Tech Stack

| Layer | Choice |
|---|---|
| Language | Rust (stable) |
| TUI | Ratatui + Crossterm |
| PTY | `portable-pty` crate (ConPTY on Windows) |
| ANSI parsing | `vte` crate |
| Async | Tokio |
| History | NDJSON (serde_json) |

---

## 📋 Status

**Pre-alpha** — Documentation and architecture phase. Implementation starts at Milestone 0.

See [roadmap.md](docs/roadmap.md) for the full delivery plan.

---

## License

MIT
