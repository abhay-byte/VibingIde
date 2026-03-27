# TODO

- Fix the window vibration loop at zoom levels above 128% by stabilizing the egui auto-scale and layout calculations.
- Trim the current `cargo check` warning list in `src/app.rs`, `src/engine`, and `src/history`.
- Persist panel session events more completely so user input and agent output are written to history as they happen.
- Add a small smoke test or scripted launcher check for common Windows agent commands like `codex`, `cmd.exe`, and `pwsh`.
