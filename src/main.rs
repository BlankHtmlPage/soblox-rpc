use std::time::{SystemTime, UNIX_EPOCH};

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

    info!("Fetching game info for universe {}...", cli.universe_id);
    let game_info = fetch_game_info(cli.universe_id).await?;
    info!("Game: {} by {}", game_info.name, game_info.creator_name);

    info!("Fetching game thumbnail...");
    let thumbnail_url = match fetch_game_thumbnail(cli.universe_id).await {
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

    let _ = drpc.on_ready(|_ctx| {
        info!("Discord RPC connected!");
    });

    drpc.start();

    // Wait for connection
    tokio::time::sleep(std::time::Duration::from_millis(500)).await;

    // Build activity
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)?
        .as_millis() as u64;

    let game_page_url = format!("https://www.roblox.com/games/{}", game_info.place_id);
    let activity_state = format!("Playing {}", game_info.name);
    let activity_details = format!("by {}", game_info.creator_name);
    let large_text = game_info.name.clone();
    let small_text = "Roblox".to_string();
    let img_url = thumbnail_url.unwrap_or_default();

    drpc.set_activity(|act| {
        act.details(&activity_state)
            .state(&activity_details)
            .timestamps(|t| t.start(now))
            .assets(|a| {
                a.large_image(&img_url)
                    .large_text(&large_text)
                    .small_image("roblox_logo")
                    .small_text(&small_text)
            })
            .append_buttons(|b| b.label("View Game Page").url(&game_page_url))
    })
    .expect("Failed to set activity");

    info!("Rich Presence set! Displaying: Playing {}", game_info.name);
    info!("Press Ctrl+C to exit.");

    // Keep alive
    tokio::signal::ctrl_c().await?;

    // Clear activity on exit
    drpc.clear_activity().ok();

    info!("Done.");
    Ok(())
}
