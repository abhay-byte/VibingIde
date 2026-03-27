# VibingIDE

Rust desktop app for running CLI coding agents side by side inside PTY-backed panels.

## Current Status

VibingIDE is no longer in the original Ratatui/TUI planning phase. The repo currently ships a native `egui` / `eframe` GUI with:

- multiple agent panels
- PTY-backed process launch
- streamed ANSI output rendering
- per-panel input boxes
- a Files sidebar
- a History sidebar that can read existing session metadata

The app is still early. History persistence is only partially wired, keyboard support is limited, and there is no built-in editor yet.

## What Works Today

- Open a project from the current directory, a positional path, or `--project`
- Launch an initial command with `--cmd`
- Run multiple CLI tools side by side
- Send stdin to each running panel
- View basic ANSI-colored output
- Load config from `~/.vibingide/config.toml` and `<project>/.vibingide/config.toml`
- Write logs to `~/.vibingide/debug.log`

## Current Limitations

- No built-in editor
- No live file watching
- No command palette
- No divider dragging or maximize flow in the active GUI
- No end-to-end session event persistence yet
- Command parsing is simple whitespace splitting, so quoted arguments with spaces are not preserved

## Quick Start

```bash
cargo run -- .
cargo run -- --project . --cmd "codex"
cargo run -- --project . --cmd "cmd.exe"
```

CLI arguments currently supported:

```text
vibingide [project]
vibingide --project <dir>
vibingide --cmd <command>
vibingide --project <dir> --cmd <command>
vibingide --help
vibingide --version
```

## Docs

- [Product snapshot](docs/specs.md)
- [Architecture](docs/architecture.md)
- [Data model](docs/data-model.md)
- [Development status](docs/roadmap.md)
- [Workflow](workflow.md)

## Development

Useful local commands:

```bash
cargo check
cargo test
```

The repo workflow is documented in [workflow.md](workflow.md). Current outstanding work is tracked in [TODO.md](TODO.md), and completed items are recorded in [DONE.md](DONE.md).

## License

MIT
