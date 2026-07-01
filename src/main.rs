use std::time::{Duration, SystemTime, UNIX_EPOCH};

use clap::Parser;
use tracing::info;

mod roblox;

use roblox::api::{fetch_game_info, fetch_game_thumbnail};

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

fn validate_inputs(cli: &Cli) -> Result<(), Box<dyn std::error::Error>> {
    if cli.universe_id == 0 {
        return Err("universe_id must be greater than 0".into());
    }
    if cli.client_id == 0 {
        return Err("client_id must be greater than 0".into());
    }
    Ok(())
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
    validate_inputs(&cli)?;

    info!("Fetching game info for universe {}...", cli.universe_id);

    let game_info = fetch_game_info(cli.universe_id).await?;
    info!("Game: {} by {}", game_info.name, game_info.creator_name);

    let thumbnail_result = fetch_game_thumbnail(cli.universe_id).await;

    let thumbnail_url = match thumbnail_result {
        Ok(url) => {
            info!("Thumbnail: {}", url);
            Some(url)
        }
        Err(e) => {
            tracing::warn!("Failed to fetch thumbnail: {:?}", e);
            None
        }
    };

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

    tokio::time::timeout(Duration::from_secs(5), rx)
        .await
        .map_err(|_| -> Box<dyn std::error::Error> { "Discord connection timed out".into() })??;

    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)?
        .as_millis() as u64;

    let game_page_url = format!("https://www.roblox.com/games/{}", game_info.place_id);
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

    tokio::signal::ctrl_c().await?;

    drpc.clear_activity().ok();

    info!("Done.");
    Ok(())
}
