# AGENTS.md

## What This Is

Rust CLI that shows Discord Rich Presence for Roblox games (Sober-style). Single binary, no tests yet.

## Quick Commands

```bash
cargo build                          # debug build
cargo build --release                # release build (use for final binary)
cargo run -- -u <UNIVERSE_ID> -c <CLIENT_ID>  # run with your Discord app
RUST_LOG=debug cargo run -- -u <ID> -c <ID>   # verbose logging
```

## Architecture

```
src/
  main.rs              # Entry point, CLI args, Discord RPC setup
  roblox/
    mod.rs             # Module declaration
    api.rs             # Roblox API calls (game info + thumbnail fetcher)
```

## Key Facts

- **Discord timestamps are milliseconds** (`as_millis()` not `as_secs()`)
- **Thumbnail API returns `imageUrl` (camelCase)** — `#[serde(rename = "imageUrl")]` is required
- **Discord shows `details` first, then `state`** in the profile — `act.details(...)` = first line, `act.state(...)` = second line
- **Client ID is required** — no default, user must create Discord app and pass `-c <ID>`
- **IPC socket fallback**: `/tmp/discord-ipc-0` for systems without `XDG_RUNTIME_DIR`
- **`on_ready` handle must be consumed**: `let _ = drpc.on_ready(...)` to avoid unused warning

## Dependencies

- `discord-presence` v3.2 (not v4+ — API changed)
- `tokio` with full async runtime
- `reqwest` for HTTP
- `clap` for CLI

## Gotchas

- No tests exist yet — `cargo test` will pass vacuously
- Thumbnail fetch can fail gracefully (fallback to empty string)
- The `universe_id` field in `GameInfo` is unused but kept for API completeness
