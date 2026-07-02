use std::time::{Duration, SystemTime, UNIX_EPOCH};

use clap::Parser;
use tracing::{info, warn};

mod roblox;

use roblox::api::{fetch_game_info, fetch_game_thumbnail};

/// How long to wait for Discord IPC to connect before giving up.
const DISCORD_CONNECT_TIMEOUT: Duration = Duration::from_secs(5);

#[derive(Parser)]
#[command(name = "soblox-rpc")]
#[command(about = "Discord Rich Presence for Roblox (Sober-style)")]
struct Cli {
    /// Roblox universe ID
    #[arg(short, long)]
    universe_id: u64,

    /// Discord application client ID
    #[arg(short, long)]
    client_id: u64,
}

impl Cli {
    /// Validate CLI inputs fail fast before any network activity.
    fn validate(&self) -> Result<(), Box<dyn std::error::Error>> {
        if self.universe_id == 0 {
            return Err("universe_id must be greater than 0".into());
        }
        if self.client_id == 0 {
            return Err("client_id must be greater than 0".into());
        }
        Ok(())
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

    info!("Fetching game info for universe {}...", cli.universe_id);

    // Run both Roblox API calls concurrently — they hit independent
    // endpoints and don't depend on each other, so this halves latency.
    let (game_info_result, thumbnail_result) = tokio::join!(
        fetch_game_info(cli.universe_id),
        fetch_game_thumbnail(cli.universe_id),
    );

    let game_info = game_info_result?;
    info!("Game: {} by {}", game_info.name, game_info.creator_name);

    let thumbnail_url = match thumbnail_result {
        Ok(url) => {
            info!("Thumbnail: {}", url);
            Some(url)
        }
        Err(e) => {
            warn!("Failed to fetch thumbnail: {:?}", e);
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
    // The API layer guarantees a non-empty URL, but guard defensively.
    let img_url = thumbnail_url.filter(|url| !url.is_empty());

    drpc.set_activity(|act| {
        act.details(format!("Playing {}", game_info.name))
            .state(format!("by {}", game_info.creator_name))
            .timestamps(|t| t.start(now))
            .assets(|a| {
                a.large_image(img_url.as_deref().unwrap_or("roblox_logo"))
                    .large_text(&game_info.name)
                    .small_image("roblox_logo")
                    .small_text("Roblox")
            })
            .append_buttons(|b| b.label("View Game Page").url(&game_page_url))
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
