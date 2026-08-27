use std::sync::LazyLock;
use std::time::Duration;

use serde::Deserialize;

const MAX_RETRIES: u32 = 3;
const RETRY_DELAY: Duration = Duration::from_secs(1);
const HTTP_TIMEOUT: Duration = Duration::from_secs(10);
/// Maximum seconds to wait when respecting a `Retry-After` header.
const MAX_RETRY_AFTER_SECS: u64 = 10;
/// How long to wait for a `Pending` thumbnail before retrying.
const THUMBNAIL_POLL_DELAY: Duration = Duration::from_millis(500);
/// Max attempts when polling for a `Pending` thumbnail to become `Completed`.
const THUMBNAIL_POLL_MAX: u32 = 3;

static SHARED_CLIENT: LazyLock<reqwest::Client> = LazyLock::new(|| {
    reqwest::Client::builder()
        .timeout(HTTP_TIMEOUT)
        .user_agent(concat!("soblox-rpc/", env!("CARGO_PKG_VERSION")))
        .build()
        .expect("Failed to build HTTP client")
});

/// Roblox game info
#[derive(Debug, Clone)]
pub struct GameInfo {
    pub name: String,
    pub creator_name: String,
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

#[derive(Debug, Deserialize, PartialEq, Eq, Clone, Copy)]
#[serde(rename_all = "PascalCase")]
enum ThumbnailState {
    Completed,
    Pending,
    Error,
    #[serde(other)]
    Unknown,
}

/// Decide whether an HTTP status code is worth retrying.
///
/// Only transient failures (server errors `5xx` and rate limiting `429`)
/// are retried. Client errors (`4xx` except `429`) fail fast since they
/// will never succeed on retry.
fn is_retryable_status(status: u16) -> bool {
    status >= 500 || status == 429
}

/// Compute the backoff delay for a given attempt (0-indexed).
///
/// Attempts 1 and 2 result in delays of `2s` and `4s` respectively
/// (attempt 0 does not sleep).
fn backoff_delay(attempt: u32) -> Duration {
    RETRY_DELAY * 2u32.saturating_pow(attempt)
}

async fn get_json<T: serde::de::DeserializeOwned>(url: &str) -> Result<T, ApiError> {
    let mut last_err: Option<ApiError> = None;
    for attempt in 0..MAX_RETRIES {
        if attempt > 0 {
            // Respect Retry-After from a 429 if we saw one (capped), otherwise exponential backoff.
            let delay = match &last_err {
                Some(ApiError::RateLimited { retry_after }) => {
                    let secs = retry_after
                        .map(|s| s.min(MAX_RETRY_AFTER_SECS))
                        .unwrap_or_else(|| backoff_delay(attempt).as_secs());
                    Duration::from_secs(secs)
                }
                _ => backoff_delay(attempt),
            };
            tokio::time::sleep(delay).await;
        }

        let resp = match SHARED_CLIENT.get(url).send().await {
            Ok(r) => r,
            Err(e) => {
                last_err = Some(ApiError::Network(e));
                continue;
            }
        };

        let status = resp.status();

        if status.as_u16() == 429 {
            // Honor Retry-After header (seconds) if present.
            let retry_after = resp
                .headers()
                .get(reqwest::header::RETRY_AFTER)
                .and_then(|v| v.to_str().ok())
                .and_then(|s| s.parse::<u64>().ok());
            last_err = Some(ApiError::RateLimited { retry_after });
            continue;
        }

        if !status.is_success() {
            last_err = Some(ApiError::Http(status.as_u16()));
            // Fail fast on non-retryable client errors (4xx except 429).
            if !is_retryable_status(status.as_u16()) {
                break;
            }
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

    Err(last_err.unwrap_or(ApiError::Http(503)))
}

/// Response for place -> universe resolution
#[derive(Debug, Deserialize)]
struct UniverseResolveResponse {
    #[serde(rename = "universeId")]
    universe_id: u64,
}

/// Resolve a place ID to its corresponding universe ID.
pub async fn resolve_place_id(place_id: u64) -> Result<u64, ApiError> {
    let url = format!(
        "https://apis.roblox.com/universes/v1/places/{}/universe",
        place_id
    );
    let resp: UniverseResolveResponse = get_json(&url).await?;
    if resp.universe_id == 0 {
        return Err(ApiError::NotFound(place_id));
    }
    Ok(resp.universe_id)
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

    let place_id = game.root_place.ok_or(ApiError::MissingPlaceId)?.id;

    Ok(GameInfo {
        name: game.name,
        creator_name: game.creator.name,
        place_id,
    })
}

/// Fetch game thumbnail URL.
///
/// Roblox's thumbnail API is asynchronous: a `Pending` state means the
/// icon is still being generated. We poll a few times before giving up.
pub async fn fetch_game_thumbnail(universe_id: u64) -> Result<String, ApiError> {
    let url = format!(
        "https://thumbnails.roblox.com/v1/games/icons?universeIds={}&size=512x512&format=Png&isCircular=false",
        universe_id
    );

    for poll in 0..THUMBNAIL_POLL_MAX {
        let thumbnails: ThumbnailResponse = get_json(&url).await?;
        match select_thumbnail(&thumbnails.data) {
            ThumbnailSelection::Ready(url) => return Ok(url),
            ThumbnailSelection::Pending if poll + 1 < THUMBNAIL_POLL_MAX => {
                tokio::time::sleep(THUMBNAIL_POLL_DELAY).await;
            }
            // No point retrying if nothing is pending.
            _ => break,
        }
    }

    Err(ApiError::NoThumbnail)
}

/// Result of selecting a thumbnail from a batch of entries.
#[derive(Debug, PartialEq, Eq)]
enum ThumbnailSelection {
    /// A completed thumbnail with a non-empty URL.
    Ready(String),
    /// At least one entry is still `Pending` (worth retrying).
    Pending,
    /// Nothing usable found.
    None,
}

/// Pick the best available thumbnail from a list of entries.
///
/// Preference order:
/// 1. First `Completed` entry with a non-empty URL.
/// 2. Otherwise, if any entry is `Pending`, signal that a retry may help.
/// 3. Otherwise, fall back to any entry with a non-empty URL.
/// 4. Finally, `None`.
///
/// Extracted as a pure function so it can be unit-tested without network access.
fn select_thumbnail(data: &[ThumbnailData]) -> ThumbnailSelection {
    // 1. Prefer a Completed entry with a non-empty URL.
    if let Some(url) = data
        .iter()
        .find(|t| t.state == ThumbnailState::Completed)
        .and_then(|t| t.image_url.as_ref())
        .filter(|url| !url.is_empty())
        .cloned()
    {
        return ThumbnailSelection::Ready(url);
    }

    // 2. If something is still Pending, a retry might yield a Completed result.
    if data.iter().any(|t| t.state == ThumbnailState::Pending) {
        return ThumbnailSelection::Pending;
    }

    // 3. Fall back to any entry with a non-empty URL.
    if let Some(url) = data
        .iter()
        .filter_map(|t| t.image_url.as_ref())
        .find(|url| !url.is_empty())
        .cloned()
    {
        return ThumbnailSelection::Ready(url);
    }

    ThumbnailSelection::None
}

#[derive(Debug, thiserror::Error)]
pub enum ApiError {
    #[error("Network error: {0}")]
    Network(#[from] reqwest::Error),

    #[error("HTTP error: status {0}")]
    Http(u16),

    #[error("Rate limited by Roblox API (retry after: {retry_after:?}s)")]
    RateLimited { retry_after: Option<u64> },

    #[error("Game not found with universe ID: {0}")]
    NotFound(u64),

    #[error("No root place ID available for this game")]
    MissingPlaceId,

    #[error("No thumbnail available")]
    NoThumbnail,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn thumb(state: ThumbnailState, url: Option<&str>) -> ThumbnailData {
        ThumbnailData {
            state,
            image_url: url.map(|s| s.to_string()),
        }
    }

    mod select_thumbnail {
        use super::*;

        #[test]
        fn completed_with_url_returns_ready() {
            let data = vec![thumb(
                ThumbnailState::Completed,
                Some("https://img.test/a.png"),
            )];
            assert_eq!(
                select_thumbnail(&data),
                ThumbnailSelection::Ready("https://img.test/a.png".to_string())
            );
        }

        #[test]
        fn completed_with_empty_url_falls_through_to_pending() {
            let data = vec![
                thumb(ThumbnailState::Completed, Some("")),
                thumb(ThumbnailState::Pending, None),
            ];
            assert_eq!(select_thumbnail(&data), ThumbnailSelection::Pending);
        }

        #[test]
        fn pending_signals_retry() {
            let data = vec![thumb(ThumbnailState::Pending, None)];
            assert_eq!(select_thumbnail(&data), ThumbnailSelection::Pending);
        }

        #[test]
        fn pending_after_completed_empty_uses_pending() {
            let data = vec![
                thumb(ThumbnailState::Completed, Some("")),
                thumb(ThumbnailState::Pending, None),
            ];
            assert_eq!(select_thumbnail(&data), ThumbnailSelection::Pending);
        }

        #[test]
        fn no_completed_but_has_url_returns_ready_fallback() {
            let data = vec![thumb(ThumbnailState::Error, Some("https://img.test/b.png"))];
            assert_eq!(
                select_thumbnail(&data),
                ThumbnailSelection::Ready("https://img.test/b.png".to_string())
            );
        }

        #[test]
        fn all_empty_urls_returns_none() {
            let data = vec![
                thumb(ThumbnailState::Completed, Some("")),
                thumb(ThumbnailState::Error, Some("")),
            ];
            assert_eq!(select_thumbnail(&data), ThumbnailSelection::None);
        }

        #[test]
        fn empty_slice_returns_none() {
            assert_eq!(select_thumbnail(&[]), ThumbnailSelection::None);
        }

        #[test]
        fn unknown_state_treated_as_non_pending() {
            let data = vec![thumb(
                ThumbnailState::Unknown,
                Some("https://img.test/c.png"),
            )];
            assert_eq!(
                select_thumbnail(&data),
                ThumbnailSelection::Ready("https://img.test/c.png".to_string())
            );
        }

        #[test]
        fn completed_preferred_over_error() {
            let data = vec![
                thumb(ThumbnailState::Error, Some("https://img.test/err.png")),
                thumb(ThumbnailState::Completed, Some("https://img.test/ok.png")),
            ];
            assert_eq!(
                select_thumbnail(&data),
                ThumbnailSelection::Ready("https://img.test/ok.png".to_string())
            );
        }

        #[test]
        fn multiple_completed_picks_first() {
            let data = vec![
                thumb(
                    ThumbnailState::Completed,
                    Some("https://img.test/first.png"),
                ),
                thumb(
                    ThumbnailState::Completed,
                    Some("https://img.test/second.png"),
                ),
            ];
            assert_eq!(
                select_thumbnail(&data),
                ThumbnailSelection::Ready("https://img.test/first.png".to_string())
            );
        }
    }

    mod retry_logic {
        use super::*;

        #[test]
        fn server_errors_are_retryable() {
            assert!(is_retryable_status(500));
            assert!(is_retryable_status(502));
            assert!(is_retryable_status(503));
            assert!(is_retryable_status(504));
        }

        #[test]
        fn rate_limit_is_retryable() {
            assert!(is_retryable_status(429));
        }

        #[test]
        fn client_errors_are_not_retryable() {
            assert!(!is_retryable_status(400));
            assert!(!is_retryable_status(401));
            assert!(!is_retryable_status(403));
            assert!(!is_retryable_status(404));
        }

        #[test]
        fn success_codes_are_not_retryable() {
            assert!(!is_retryable_status(200));
            assert!(!is_retryable_status(201));
            assert!(!is_retryable_status(204));
        }

        #[test]
        fn backoff_delay_attempt_1_and_2() {
            assert_eq!(backoff_delay(1), Duration::from_secs(2));
            assert_eq!(backoff_delay(2), Duration::from_secs(4));
        }
    }

    mod url_parsing {
        use crate::Cli;

        #[test]
        fn full_game_url_extracts_place_id() {
            let url = "https://www.roblox.com/games/1818/Classic-Crossroads";
            assert_eq!(Cli::parse_place_id_from_url(url).unwrap(), 1818);
        }

        #[test]
        fn url_without_title_extracts_place_id() {
            let url = "https://www.roblox.com/games/4252370513";
            assert_eq!(Cli::parse_place_id_from_url(url).unwrap(), 4252370513);
        }

        #[test]
        fn relative_path_extracts_place_id() {
            let url = "/games/920587237/Adopt-Me";
            assert_eq!(Cli::parse_place_id_from_url(url).unwrap(), 920587237);
        }

        #[test]
        fn raw_numeric_string_parses() {
            assert_eq!(Cli::parse_place_id_from_url("12345").unwrap(), 12345);
        }

        #[test]
        fn invalid_url_fails() {
            assert!(Cli::parse_place_id_from_url("https://example.com/not-roblox").is_err());
        }

        #[test]
        fn zero_place_id_fails() {
            assert!(Cli::parse_place_id_from_url("https://www.roblox.com/games/0/test").is_err());
        }
    }
}
