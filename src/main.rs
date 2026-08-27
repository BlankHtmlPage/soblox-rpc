use std::time::{Duration, SystemTime, UNIX_EPOCH};

use clap::Parser;
use tracing::{info, warn};

mod roblox;

use roblox::api::{fetch_game_info, fetch_game_thumbnail, resolve_place_id};

/// How long to wait for Discord IPC to connect before giving up.
const DISCORD_CONNECT_TIMEOUT: Duration = Duration::from_secs(5);
/// Maximum time to spend attempting to fetch an optional thumbnail.
const THUMBNAIL_FETCH_TIMEOUT: Duration = Duration::from_secs(5);

#[derive(Parser)]
#[command(name = "soblox-rpc")]
#[command(version)]
#[command(about = "Discord Rich Presence for Roblox (Sober-style)")]
struct Cli {
    /// Roblox universe ID
    #[arg(short, long, conflicts_with_all = ["place_id", "url"])]
    universe_id: Option<u64>,

    /// Roblox place ID (resolves universe ID automatically)
    #[arg(short, long, conflicts_with = "url")]
    place_id: Option<u64>,

    /// Roblox game URL (e.g. https://www.roblox.com/games/1818/Classic-Crossroads)
    #[arg(long)]
    url: Option<String>,

    /// Discord application client ID
    #[arg(short, long)]
    client_id: u64,
}

impl Cli {
    /// Validate CLI inputs fail fast before any network activity.
    fn validate(&self) -> Result<(), Box<dyn std::error::Error>> {
        if self.universe_id.is_none() && self.place_id.is_none() && self.url.is_none() {
            return Err("Must provide one of: --universe-id, --place-id, or --url".into());
        }
        if self.universe_id == Some(0) {
            return Err("universe_id must be greater than 0".into());
        }
        if self.place_id == Some(0) {
            return Err("place_id must be greater than 0".into());
        }
        if self.client_id == 0 {
            return Err("client_id must be greater than 0".into());
        }
        Ok(())
    }

    /// Extract a place ID from a Roblox game URL.
    fn parse_place_id_from_url(raw_url: &str) -> Result<u64, Box<dyn std::error::Error>> {
        let trimmed = raw_url.trim();
        // Support full URLs like https://www.roblox.com/games/1818/name
        // or path-only like /games/1818/name or just raw place ID strings.
        let path = if let Some(idx) = trimmed.find("/games/") {
            &trimmed[idx + "/games/".len()..]
        } else {
            trimmed
        };
        let numeric_part = path.split('/').next().unwrap_or("").trim();
        let place_id = numeric_part
            .parse::<u64>()
            .map_err(|_| format!("Could not extract a valid numeric place ID from: {raw_url}"))?;
        if place_id == 0 {
            return Err("place ID in URL must be greater than 0".into());
        }
        Ok(place_id)
    }

    /// Resolve the final universe ID from whatever input option was provided.
    async fn resolve_universe_id(&self) -> Result<u64, Box<dyn std::error::Error>> {
        if let Some(uid) = self.universe_id {
            return Ok(uid);
        }
        let pid = if let Some(pid) = self.place_id {
            pid
        } else if let Some(url) = &self.url {
            Self::parse_place_id_from_url(url)?
        } else {
            unreachable!("Guarded by validate()");
        };

        info!("Resolving universe ID for place {}...", pid);
        let uid = resolve_place_id(pid).await?;
        info!("Resolved universe ID: {}", uid);
        Ok(uid)
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .with_target(false)
        .with_level(true)
        .init();

    let cli = Cli::parse();
    cli.validate()?;

    let universe_id = cli.resolve_universe_id().await?;

    info!("Fetching game info for universe {}...", universe_id);

    // Run game info and thumbnail fetches concurrently. The thumbnail fetch
    // is wrapped in a timeout so slow thumbnail polling cannot block startup.
    let (game_info_result, thumbnail_result) = tokio::join!(
        fetch_game_info(universe_id),
        tokio::time::timeout(THUMBNAIL_FETCH_TIMEOUT, fetch_game_thumbnail(universe_id)),
    );

    let game_info = game_info_result?;
    info!("Game: {} by {}", game_info.name, game_info.creator_name);

    let thumbnail_url = match thumbnail_result {
        Ok(Ok(url)) => {
            info!("Thumbnail: {}", url);
            Some(url)
        }
        Ok(Err(e)) => {
            warn!("Failed to fetch thumbnail: {:?}", e);
            None
        }
        Err(_) => {
            warn!("Thumbnail fetch timed out, falling back to default asset");
            None
        }
    };

    // Capture the start timestamp *before* connecting to Discord so the
    // "elapsed" timer reflects when the user launched the tool, not when
    // the IPC handshake completed.
    let now = SystemTime::now().duration_since(UNIX_EPOCH)?.as_millis() as u64;

    info!("Connecting to Discord...");
    let mut drpc = discord_presence::Client::new(cli.client_id);

    let (tx, rx) = tokio::sync::oneshot::channel();
    let tx = std::sync::Mutex::new(Some(tx));
    let _ready_handle = drpc.on_ready(move |_ctx| {
        info!("Discord RPC connected!");
        if let Some(tx) = tx.lock().unwrap().take() {
            let _ = tx.send(());
        }
    });
    _ready_handle.persist();

    drpc.start();

    tokio::time::timeout(DISCORD_CONNECT_TIMEOUT, rx)
        .await
        .map_err(|_| -> Box<dyn std::error::Error> { "Discord connection timed out".into() })??;

    let game_page_url = format!("https://www.roblox.com/games/{}", game_info.place_id);

    drpc.set_activity(|act| {
        act.details(format!("Playing {}", game_info.name))
            .state(format!("by {}", game_info.creator_name))
            .timestamps(|t| t.start(now))
            .assets(|a| {
                a.large_image(thumbnail_url.as_deref().unwrap_or("roblox_logo"))
                    .large_text(&game_info.name)
                    .small_image("roblox_logo")
                    .small_text("Roblox")
            })
            .append_buttons(|b| b.label("See game page").url(&game_page_url))
    })
    .map_err(|e| format!("Failed to set activity: {e}"))?;

    info!("Rich Presence set! Displaying: Playing {}", game_info.name);
    info!("Press Ctrl+C to exit.");

    // Wait for a termination signal. On Unix we listen for both SIGINT
    // (Ctrl+C) and SIGTERM (sent by process managers / desktop envs) so
    // we always get a chance to clear Discord activity cleanly instead
    // of leaving a stale presence behind.
    wait_for_termination_signal().await?;

    drpc.clear_activity().ok();

    info!("Done.");
    Ok(())
}

/// Block until the process receives a termination signal.
///
/// On Unix this listens for both `SIGINT` and `SIGTERM`. On other
/// platforms (Windows) only `Ctrl+C` (`SIGINT` equivalent) is handled.
async fn wait_for_termination_signal() -> Result<(), Box<dyn std::error::Error>> {
    #[cfg(unix)]
    {
        use tokio::signal::unix::{SignalKind, signal};

        let mut sigint = signal(SignalKind::interrupt())?;
        let mut sigterm = signal(SignalKind::terminate())?;

        tokio::select! {
            _ = sigint.recv() => {}
            _ = sigterm.recv() => {}
        }
    }
    #[cfg(not(unix))]
    {
        tokio::signal::ctrl_c().await?;
    }

    Ok(())
}
