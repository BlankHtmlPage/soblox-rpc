use std::time::Duration;

use serde::Deserialize;

const MAX_RETRIES: u32 = 3;
const RETRY_DELAY: Duration = Duration::from_secs(1);

/// Roblox game info
#[derive(Debug, Clone)]
pub struct GameInfo {
    pub name: String,
    pub creator_name: String,
    pub universe_id: u64,
    pub place_id: u64,
}

/// Roblox API response for games
#[derive(Debug, Deserialize)]
struct GamesResponse {
    data: Vec<GameData>,
}

#[derive(Debug, Deserialize)]
struct GameData {
    name: String,
    creator: Creator,
    id: u64,
    root_place: Option<RootPlace>,
}

#[derive(Debug, Deserialize)]
struct Creator {
    name: String,
}

#[derive(Debug, Deserialize)]
struct RootPlace {
    id: u64,
}

/// Roblox API response for thumbnails
#[derive(Debug, Deserialize)]
struct ThumbnailResponse {
    data: Vec<ThumbnailData>,
}

#[derive(Debug, Deserialize)]
struct ThumbnailData {
    state: ThumbnailState,
    #[serde(rename = "imageUrl")]
    image_url: Option<String>,
}

#[derive(Debug, Deserialize, PartialEq)]
#[serde(rename_all = "PascalCase")]
enum ThumbnailState {
    Completed,
    Pending,
    Error,
    #[serde(other)]
    Unknown,
}

fn build_client() -> Result<reqwest::Client, ApiError> {
    reqwest::Client::builder()
        .timeout(Duration::from_secs(10))
        .build()
        .map_err(ApiError::Network)
}

async fn get_json<T: serde::de::DeserializeOwned>(url: &str) -> Result<T, ApiError> {
    let client = build_client()?;

    let mut last_err = None;
    for attempt in 0..MAX_RETRIES {
        if attempt > 0 {
            tokio::time::sleep(RETRY_DELAY).await;
        }

        let resp = match client.get(url).send().await {
            Ok(r) => r,
            Err(e) => {
                last_err = Some(ApiError::Network(e));
                continue;
            }
        };

        if !resp.status().is_success() {
            last_err = Some(ApiError::Http(resp.status().as_u16()));
            continue;
        }

        match resp.json::<T>().await {
            Ok(val) => return Ok(val),
            Err(e) => {
                last_err = Some(ApiError::Network(e));
                continue;
            }
        }
    }

    Err(last_err.expect("retry loop must set last_err"))
}

/// Fetch game info from Roblox API
pub async fn fetch_game_info(universe_id: u64) -> Result<GameInfo, ApiError> {
    let url = format!(
        "https://games.roblox.com/v1/games?universeIds={}",
        universe_id
    );

    let games: GamesResponse = get_json(&url).await?;

    let game = games
        .data
        .into_iter()
        .next()
        .ok_or_else(|| ApiError::NotFound(universe_id))?;

    let place_id = game
        .root_place
        .ok_or(ApiError::MissingPlaceId)?
        .id;

    Ok(GameInfo {
        name: game.name,
        creator_name: game.creator.name,
        universe_id: game.id,
        place_id,
    })
}

/// Fetch game thumbnail URL
pub async fn fetch_game_thumbnail(universe_id: u64) -> Result<String, ApiError> {
    let url = format!(
        "https://thumbnails.roblox.com/v1/games/icons?universeIds={}&size=512x512&format=Png&isCircular=false",
        universe_id
    );

    let thumbnails: ThumbnailResponse = get_json(&url).await?;

    thumbnails
        .data
        .iter()
        .find(|t| t.state == ThumbnailState::Completed && t.image_url.is_some())
        .or_else(|| thumbnails.data.iter().find(|t| t.image_url.is_some()))
        .and_then(|t| t.image_url.clone())
        .ok_or(ApiError::NoThumbnail)
}

#[derive(Debug, thiserror::Error)]
pub enum ApiError {
    #[error("Network error: {0}")]
    Network(#[from] reqwest::Error),

    #[error("HTTP error: status {0}")]
    Http(u16),

    #[error("Game not found with universe ID: {0}")]
    NotFound(u64),

    #[error("No root place ID available for this game")]
    MissingPlaceId,

    #[error("No thumbnail available")]
    NoThumbnail,
}
