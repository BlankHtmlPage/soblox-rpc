use serde::Deserialize;

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
    state: String,
    #[serde(rename = "imageUrl")]
    image_url: Option<String>,
}

/// Fetch game info from Roblox API
pub async fn fetch_game_info(universe_id: u64) -> Result<GameInfo, ApiError> {
    let url = format!(
        "https://games.roblox.com/v1/games?universeIds={}",
        universe_id
    );

    let resp = reqwest::get(&url).await.map_err(ApiError::Network)?;

    let games: GamesResponse = resp.json().await.map_err(ApiError::Network)?;

    let game = games
        .data
        .into_iter()
        .next()
        .ok_or_else(|| ApiError::NotFound(universe_id))?;

    let place_id = game.root_place.map(|r| r.id).unwrap_or(universe_id);

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

    let resp = reqwest::get(&url).await.map_err(ApiError::Network)?;

    let thumbnails: ThumbnailResponse = resp.json().await.map_err(ApiError::Network)?;

    // Try Completed first, then any with a URL
    thumbnails
        .data
        .iter()
        .find(|t| t.state == "Completed" && t.image_url.is_some())
        .or_else(|| thumbnails.data.iter().find(|t| t.image_url.is_some()))
        .and_then(|t| t.image_url.clone())
        .ok_or(ApiError::NoThumbnail)
}

#[derive(Debug, thiserror::Error)]
pub enum ApiError {
    #[error("Network error: {0}")]
    Network(#[from] reqwest::Error),

    #[error("Game not found with universe ID: {0}")]
    NotFound(u64),

    #[error("No thumbnail available")]
    NoThumbnail,
}
