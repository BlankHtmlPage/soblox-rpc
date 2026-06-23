# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [1.0.0] - 2026-06-23

### Added

- Discord Rich Presence for Roblox games
- Shows "Playing Roblox" on Discord profile
- Displays game name and creator
- Live elapsed time counter
- Game thumbnail as large image
- Roblox logo as small image
- "View Game Page" button
- CLI with `--universe-id` and `--client-id` options
- Support for Bloxstrap's Discord Application ID
- Support for custom Discord Application IDs
- Cross-platform support (Linux, macOS, Windows)
- GitHub Actions for debug and release builds
- Auto-fetches game info from Roblox API
- Auto-fetches game thumbnails from Roblox API

### Fixed

- IPC socket path fallback for systems without `XDG_RUNTIME_DIR`
- Thumbnail API response parsing (camelCase field names)
