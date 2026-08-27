# soblox-rpc

[![Debug Build](https://github.com/BlankHtmlPage/soblox-rpc/actions/workflows/debug.yml/badge.svg)](https://github.com/BlankHtmlPage/soblox-rpc/actions/workflows/debug.yml)
[![Release](https://img.shields.io/github/v/release/BlankHtmlPage/soblox-rpc)](https://github.com/BlankHtmlPage/soblox-rpc/releases)
[![License: MIT](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)
[![Discussions](https://img.shields.io/github/discussions/BlankHtmlPage/soblox-rpc)](https://github.com/BlankHtmlPage/soblox-rpc/discussions)

Sober-style Discord Rich Presence for Roblox. Shows what Roblox game you are playing on your Discord profile.

> **Disclaimer:** This project is not affiliated with, endorsed by, or associated with Roblox Corporation or the Sober project. It is an independent utility that connects to your local Discord desktop client via the standard Discord Rich Presence IPC socket.

## Features

- Displays "Playing Roblox" status on your Discord profile
- Shows current game name and creator
- Live elapsed time counter
- Game thumbnail as large image (falls back to default Roblox logo if unavailable)
- Roblox logo as small image
- "View Game Page" button linking directly to the Roblox game page
- Supports direct game URLs, place IDs, or universe IDs

## Prerequisites

- Rust 1.85+ (install via [rustup](https://rustup.rs/)) — edition 2024 requires Rust 1.85 or newer
- Discord desktop client running (Stable, PTB, and Canary supported)
  - **Linux Flatpak note:** Sandboxed Flatpak Discord cannot always access the host IPC socket directly. If presence fails to connect, grant Flatpak socket access or run Discord natively.

## Discord Application Setup

Before using this tool, create a Discord application once:

1. Go to the [Discord Developer Portal](https://discord.com/developers/applications)
2. Click **New Application**
3. Name it **`Roblox`** (this is what appears as the top line in your profile)
4. Navigate to **Rich Presence** -> **Art Assets**
5. Click **Add Image(s)** and upload [`app_assets/roblox_logo.png`](app_assets/roblox_logo.png)
6. Name the asset **`roblox_logo`** (must match exactly)
7. Navigate to **General Information** and copy the **Application ID**

## Installation

### From Source

```bash
git clone https://github.com/BlankHtmlPage/soblox-rpc.git
cd soblox-rpc
cargo build --release
```

The compiled binary will be at `target/release/soblox-rpc`.

### Cargo Install

```bash
cargo install --git https://github.com/BlankHtmlPage/soblox-rpc.git
```

### Pre-built Binaries

Download pre-compiled binaries from [GitHub Releases](https://github.com/BlankHtmlPage/soblox-rpc/releases).

## Usage

You can provide a game URL, a place ID, or a universe ID:

```bash
# Using a game URL (easiest):
soblox-rpc --url https://www.roblox.com/games/4252370513/Brookhaven-RP -c <CLIENT_ID>

# Using a place ID (from the game URL):
soblox-rpc -p 4252370513 -c <CLIENT_ID>

# Using a universe ID:
soblox-rpc -u 1603317378 -c <CLIENT_ID>
```

### Verbose Logging

To see debug logs and network requests:

```bash
RUST_LOG=debug soblox-rpc -p 4252370513 -c <CLIENT_ID>
```

### Finding the Universe ID Manually (Optional)

If you prefer passing `-u` directly, query the universe ID using the place ID:

```bash
curl "https://apis.roblox.com/universes/v1/places/PLACE_ID/universe"
```

### CLI Options

| Flag | Description | Required |
|------|-------------|----------|
| `--url <URL>` | Roblox game URL (resolves universe ID automatically) | one of these |
| `-p, --place-id <ID>` | Roblox place ID (resolves universe ID automatically) | one of these |
| `-u, --universe-id <ID>` | Roblox universe ID | one of these |
| `-c, --client-id <ID>` | Discord application client ID | yes |
| `-V, --version` | Print version information | - |
| `-h, --help` | Print help | - |

## Building

### Debug Build

```bash
cargo build
```

### Release Build

```bash
cargo build --release
```

## Cross-Compilation

```bash
# Linux
cargo build --release --target x86_64-unknown-linux-gnu

# macOS
cargo build --release --target x86_64-apple-darwin

# Windows
cargo build --release --target x86_64-pc-windows-msvc
```

## How It Works

1. Resolves the game URL or place ID to a universe ID via the Roblox API (if not already provided)
2. Fetches game metadata (title, creator, root place ID) from the Roblox API
3. Fetches the game icon thumbnail concurrently with a timeout fallback
4. Connects to the local Discord desktop client via the standard IPC socket
5. Sends the `SET_ACTIVITY` payload to display your presence
6. Clears the Discord presence cleanly upon receiving `SIGINT` (Ctrl+C) or `SIGTERM`

## License

MIT License -- see [LICENSE](LICENSE) for details.

## Funding

If you find this useful, you can [support development](https://bhp.qzz.io/donate/).

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md) for contribution guidelines.
See [CHANGELOG.md](CHANGELOG.md) for version history.
See [SECURITY.md](SECURITY.md) for reporting vulnerabilities.

## Acknowledgments

- [discord-presence](https://crates.io/crates/discord-presence) -- Rust Discord RPC client library
