# CLAUDE.md

## Build & Run

- `cd src-tauri && /Users/korenkrita/.cargo/bin/cargo check` — verify Rust compilation
- `cd src-tauri && /Users/korenkrita/.cargo/bin/cargo test` — run 34 unit tests
- `npx tsc --noEmit` — TypeScript type check (run from project root)
- `export PATH="$HOME/.cargo/bin:$PATH" && pnpm tauri dev` — full app with hot reload
- `cargo` is NOT in default shell PATH for subshells; use full path or export

## Environment Quirks

- macOS `dirs::config_dir()` → `~/Library/Application Support` (NOT `~/.config`)
- Real DB: `~/Library/Application Support/cc-monitor/data.db`
- Real config: `~/Library/Application Support/cc-monitor/settings.json`
- SQLite `.schema` won't show ALTER TABLE columns — use `PRAGMA table_info(requests)`
- `cargo run` alone won't work for Tauri apps (needs GUI context) — use `pnpm tauri dev`

## Architecture

- Rust backend (Tauri 2) + React 19 frontend
- Watcher thread polls `~/.claude/projects/` and `~/.codex/sessions/` every 500ms
- File list rescanned every ~15s; config reloaded on mtime change
- DB uses WAL mode; Mutex<Connection> for thread safety
- Project paths encoded as `-Users-foo-bar` format (both Claude and Codex sources)

## Gotchas

- `TrayIconBuilder::new()` has no ID — use `::with_id("main")` or `tray_by_id()` returns None silently
- `notify` crate FSEvents unreliable on macOS for JSONL tailing — polling is intentional, don't switch back
- Watcher rejects messages older than 1h before app start (prevents stale data from modified old files)
- Tauri icon PNGs must be RGBA — needs `image-png` feature in Cargo.toml for `Image::from_bytes()`
- `set_icon(None)` after tray build removes icon image but macOS still allocates ~16px padding
- `<synthetic>` model appears in JSONL from context compaction — parser filters `model.starts_with('<')`

## Testing

- Unit tests use `Database::new_in_memory()` for isolation
- No integration test framework; E2E is manual via `pnpm tauri dev`
- Always run `cargo test` + `npx tsc --noEmit` before committing
