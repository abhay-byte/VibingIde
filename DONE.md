# DONE

- 2026-03-25: Fixed the Windows agent launch path so VibingIDE now resolves native executables before shim files, preserves a safer child environment, records real child PIDs, and kills/waits for processes more cleanly. Commit: `9e147f1`
- 2026-03-25: Added repo workflow tracking files (`WORKFLOW.md`, `TODO.md`, `DONE.md`) plus a GitHub Actions CI workflow for `cargo check` and Windows test runs. Commit: `83157a4`
- 2026-03-25: Added `scripts\launch-vibingide.ps1` and `scripts\update-shortcuts.ps1`, then refreshed the Desktop and Startup `VibingIDE.lnk` shortcuts to point at the managed launcher.
- 2026-03-25: Added Linux launcher support with `scripts/launch-vibingide.sh` and `scripts/update-linux-shortcuts.sh`, which can create desktop, application-menu, and autostart `.desktop` entries for terminal-based launches.
- 2026-03-25: Debugged the remaining Windows command failure and normalized project working directories away from `\\?\...` verbatim paths so spawned tools now inherit a normal `C:\...` cwd. Also fixed the PowerShell launcher when no arguments are passed and refreshed the Windows shortcuts.
