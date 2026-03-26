# TODO

- Fix portrait mode panel sizing so the focused terminal fills the remaining viewport instead of leaving a black region on tall or narrow windows.
- Sync `README.md` and docs with the current `egui` / `eframe` GUI implementation instead of the older Ratatui/TUI plan.
- Trim the current `cargo check` warning list in `src/app.rs`, `src/engine`, and `src/history`.
- Persist panel session events more completely so user input and agent output are written to history as they happen.
- Add a small smoke test or scripted launcher check for common Windows agent commands like `codex`, `cmd.exe`, and `pwsh`.
