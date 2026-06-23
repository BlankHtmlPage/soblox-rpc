# roblox-rpc

Discord Rich Presence for Roblox — Sober-style. Shows what Roblox game you're playing on your Discord profile.

<!-- Replace with actual screenshot -->
![Roblox RPC Screenshot](docs/screenshot.png)

## Features

- Shows "Playing Roblox" on your Discord profile
- Displays the game name you're currently playing
- Shows the game creator's name
- Live elapsed time counter
- Game thumbnail as large image
- Roblox logo as small image
- "View Game Page" button linking to the Roblox game page

## Display Format

```
Playing ⚙️
           Roblox
           Playing 🏰 Block Tales [DEMO 5]
           by Spaceman Moonbase
           ⏱ 2:50:07
```

## Prerequisites

- Rust (install via [rustup](https://rustup.rs/))
- Discord desktop client running (PTB supported)

## Installation

### From Source

```bash
git clone https://github.com/yourusername/roblox-rpc.git
cd roblox-rpc
cargo build --release
```

The binary will be at `target/release/roblox-rpc`.

### Pre-built Binaries

Download the latest release from [GitHub Releases](https://github.com/yourusername/roblox-rpc/releases).

## Usage

```bash
roblox-rpc --universe-id <UNIVERSE_ID>
```

### Finding the Universe ID

Every Roblox game has a place ID in its URL. Use the Roblox API to get the universe ID:

```bash
# Replace PLACE_ID with the number from the game URL
curl "https://apis.roblox.com/universes/v1/places/PLACE_ID/universe"
```

Or run this in your browser console on the game page:

```javascript
fetch(`https://apis.roblox.com/universes/v1/places/${window.location.pathname.split('/')[2]}/universe`)
  .then(r => r.json())
  .then(d => console.log(d.universeId))
```

### Examples

```bash
# Brookhaven RP
cargo run -- -u 4252370513

# Block Tales
cargo run -- -u 5678284602

# Use custom Discord Application ID
cargo run -- -u 4252370513 -c 123456789012345678
```

### CLI Options

| Flag | Description | Default |
|------|-------------|---------|
| `-u, --universe-id` | Roblox universe ID (required) | — |
| `-c, --client-id` | Discord application client ID | `bloxstrap` |
| `-h, --help` | Print help | — |

### Discord Application ID

By default, the app uses Bloxstrap's Discord Application ID (`1005469189907173486`). You can:

- Use `bloxstrap` (default) — uses Bloxstrap's verified app
- Provide your own Discord Application ID from the [Discord Developer Portal](https://discord.com/developers/applications)

To create your own Discord app:
1. Go to [Discord Developer Portal](https://discord.com/developers/applications)
2. Click "New Application"
3. Name it **`Roblox`** (exact name matters)
4. Go to **Rich Presence** → **Art Assets**
5. Upload a Roblox logo image and name it `roblox_logo`
6. Copy the Application ID from **General Information**

## Building

### Debug Build

```bash
cargo build
# Binary: target/debug/roblox-rpc
```

### Release Build

```bash
cargo build --release
# Binary: target/release/roblox-rpc
```

### Optimized Release

```bash
cargo build --release
strip target/release/roblox-rpc
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

1. Fetches game info (name, creator) from the Roblox API
2. Fetches game thumbnail from the Roblox API
3. Connects to Discord's local IPC socket
4. Sends `SET_ACTIVITY` with the game data
5. Handles heartbeats to maintain the connection
6. Clears activity on exit

## License

MIT License — see [LICENSE](LICENSE) for details.

## Acknowledgments

- [Bloxstrap](https://github.com/bloxstraplabs/bloxstrap) — for the Discord Application ID
- [discord-presence](https://crates.io/crates/discord-presence) — Rust Discord RPC library
