# VibingIDE - Development Status

Last reviewed: 2026-03-27

This file replaces the older milestone plan that assumed implementation had not started yet. The repository already contains a working GUI shell and PTY integration, so the roadmap is now organized by actual status.

## 1. Shipped or mostly shipped

- Native desktop shell built with `eframe` and `egui`
- Project opening from current directory, positional path, or `--project`
- Optional initial agent launch via `--cmd`
- PTY-backed agent processes using `portable-pty`
- Multiple side-by-side agent panels
- Streaming PTY output into a per-panel ring buffer
- Basic ANSI color/style rendering
- Per-panel input bars for sending stdin
- Left sidebar with Files and History tabs
- Project tree scan with simple `.gitignore` filtering
- Config loading and validation
- Logging to `~/.vibingide/debug.log`
- Windows command resolution tests for executable/script launch behavior

## 2. Partial features that still need wiring

- Session history persistence
  - Event schema and storage helpers exist
  - Runtime does not yet append events or update `index.json`
- History sidebar
  - Existing `index.json` entries can be displayed
  - New sessions are not currently added by the app
- Keyboard navigation
  - The codebase contains keybind schemas and helper modules
  - The active egui app currently wires only a small subset of shortcuts
- Config application
  - Config parsing is broader than current runtime usage
  - Project config replacement is coarse rather than field-by-field merge
- PTY resize support
  - Resize methods exist in the engine
  - The active GUI is not yet driving them end to end

## 3. Recommended next milestones

### Milestone A - Finish history end to end

- Create `SessionMeta` when a panel is launched
- Write `session_start`, `user_input`, `agent_output`, and `session_end`
- Save and refresh `index.json`
- Make the History tab reflect sessions created in the current run

### Milestone B - Close the GUI interaction gaps

- Wire the intended keyboard shortcuts in the egui app
- Add panel focus cycling and keyboard close actions
- Hook PTY resize to actual panel/window changes
- Replace the placeholder 1x1 app icon

### Milestone C - Improve project navigation

- Add live file refresh or a real file watcher
- Improve `.gitignore` handling beyond simple filename matching
- Support opening files or copying paths directly from the tree
- Add recent projects and project switching

### Milestone D - Polish and packaging

- Richer ANSI support
- Better command parsing for quoted arguments and paths with spaces
- Command palette and higher-level app actions
- Release packaging and install instructions

## 4. Backlog

- Session replay viewer
- Editor pane
- Search across project and history
- Theming beyond the current hardcoded visual style
- Better accessibility and keyboard-only navigation
- CI and release automation documentation cleanup
