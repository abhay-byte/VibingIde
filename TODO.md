# TODO

- Sync `README.md` and docs with the current `egui` / `eframe` GUI implementation instead of the older Ratatui/TUI plan.
- Trim the current `cargo check` warning list in `src/app.rs`, `src/engine`, and `src/history`.
- Persist panel session events more completely so user input and agent output are written to history as they happen.
- Add a small smoke test or scripted launcher check for common Windows agent commands like `codex`, `cmd.exe`, and `pwsh`.
- Add automated GUI smoke coverage for direct PTY keyboard input and alternate-screen terminal apps on Linux.
