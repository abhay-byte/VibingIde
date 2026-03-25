# Workflow

This repo uses a simple issue flow so work stays visible and gets pushed in small steps.

## Loop

1. Pick one issue at a time from `TODO.md`.
2. Implement the fix or feature in a focused branch or directly on `main`.
3. Verify the change with the smallest useful checks first, then `cargo check`.
4. Move the finished item from `TODO.md` to `DONE.md` with the date and commit hash.
5. Commit immediately after that issue is complete.
6. Push immediately after the commit lands.

## Rules

- Keep issues small enough to finish in one commit when possible.
- Record follow-up work in `TODO.md` instead of hiding it in your head.
- If a task uncovers more work, finish the current issue first and add the rest as new TODO items.
- If a change affects launch, Windows behavior, or release flow, refresh the local shortcuts with `scripts\update-shortcuts.ps1`.

## Definition Of Done

- The code change is implemented.
- The relevant verification command was run.
- `DONE.md` was updated.
- A commit was created.
- The commit was pushed.
