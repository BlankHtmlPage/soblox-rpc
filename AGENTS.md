# AGENTS.md

## What This Is

Rust CLI that shows Discord Rich Presence for Roblox games (Sober-style). Single binary.

## Quick Commands

```bash
cargo build                                             # debug build
cargo build --release                                   # release build (use for final binary)
cargo test                                              # run unit tests
cargo run -- -u <UNIVERSE_ID> -c <CLIENT_ID>            # run with universe ID
cargo run -- -p <PLACE_ID> -c <CLIENT_ID>               # run with place ID
cargo run -- --url <GAME_URL> -c <CLIENT_ID>            # run with game URL
RUST_LOG=debug cargo run -- -u <ID> -c <ID>             # verbose logging
```

## Architecture

```
src/
  main.rs              # Entry point, CLI args, input resolution, Discord RPC setup
  roblox/
    mod.rs             # Module declaration
    api.rs             # Roblox API calls (game info, thumbnail, place resolution)
```

## Key Facts

- **Discord timestamps are milliseconds** (`as_millis()` not `as_secs()`)
- **Thumbnail API returns `imageUrl` (camelCase)** — `#[serde(rename = "imageUrl")]` is required
- **Client ID is required** — no default, user must create Discord app and pass `-c <ID>`
- **IPC socket fallback** (via `discord-presence`): `/tmp/discord-ipc-0` for systems without `XDG_RUNTIME_DIR`
- **Thumbnail fallback**: falls back to `roblox_logo` Discord asset if API thumbnail fails or times out

## Dependencies

- `discord-presence` v3.2 (not v4+ — API changed)
- `tokio` with full async runtime
- `reqwest` for HTTP
- `clap` for CLI

## Gotchas

- Thumbnail fetch can fail gracefully (fallback to default `roblox_logo` asset)
- Unit tests cover thumbnail selection, retry status logic, and URL place ID parsing (`cargo test`)
- Flatpak Discord on Linux may require permissions to access the host IPC socket

