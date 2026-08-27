# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [1.0.0] - 2026-08-27

### Added

- Discord Rich Presence for Roblox games — "Playing Roblox" on Discord profile
- Displays game name, creator, live elapsed time, game thumbnail, Roblox logo, and "View Game Page" button
- CLI with `--universe-id`, `--place-id` (`-p`), `--url`, `--client-id` (`-c`), and `--version`
- Game URL and place ID input with automatic universe resolution
- Support for custom Discord Application IDs
- Cross-platform support (Linux, macOS, Windows)
- Auto-fetches game info and thumbnails from Roblox API

### Fixed

- IPC socket fallback for systems without `XDG_RUNTIME_DIR` (via `discord-presence`)
- Thumbnail API camelCase parsing, `Pending` thumbnail polling, and bounded 5s thumbnail fetch (prevents ~110s stalls)
- Retry logic fails fast on non-retryable 4xx, honors `Retry-After` (capped at 10s), and uses exponential backoff (2s then 4s)
- `expect()` panics on `get_json`/`set_activity` replaced with safe fallbacks
- SIGTERM handling, accurate elapsed timer, `universe_id`/`place_id` fallback, 10s HTTP timeout, and `map_or` clippy fix

### Changed

- `fetch_game_info` and `fetch_game_thumbnail` run concurrently; `validate_inputs` → `Cli::validate()`; timeouts → named constants
- Thumbnail selection extracted to `select_thumbnail()` with unit tests
- Replaced `once_cell` with `std::sync::LazyLock`; removed `wiremock`/`serde_json` direct deps

### Security

- `User-Agent` header, SHA-pinned GitHub Actions, tag validation, `overflow-checks`, and `cargo-audit` in CI (`clippy`/`fmt`/`test` enforced)
- `h2` bumped to 0.4.19 fixing RUSTSEC-2026-0258
