use std::time::Duration;

use axum::Json;
use axum::extract::State;
use axum::http::StatusCode;
use axum::routing::{Router, get};
use futures_util::TryFutureExt;
use moka::future::Cache;
use reqwest::{Client, Response};
use serde::{Deserialize, Serialize};
use serde_json::Value;

const CACHE_KEY: &str = "IS_LIVE_TWITCH_API_CACHE_KEY";
const CACHE_TTL: Duration = Duration::from_secs(60);

#[derive(Clone)]
pub struct TwitchState {
    client: Client,
    cache: Cache<&'static str, TwitchApiResponse>,
}

impl TwitchState {
    pub fn new() -> Self {
        Self {
            client: Client::new(),
            cache: Cache::builder().time_to_live(CACHE_TTL).build(),
        }
    }
}

#[derive(Clone, Serialize, Deserialize)]
pub struct TwitchApiResponse {
    is_live: bool,
}

async fn get_is_live_from_response(response: Response) -> Result<bool, String> {
    response
        .text()
        .await
        .map_err(|e| e.to_string())
        .and_then(|body| serde_json::from_str::<Value>(body.as_str()).map_err(|e| e.to_string()))
        .and_then(|json_body| {
            json_body
                .pointer("/data/user/stream")
                .cloned()
                .ok_or("Unable to determine `is_live` status.".to_string())
        })
        .map(|val| !val.is_null())
}

async fn fetch_twitch_api_response(client: &Client) -> Result<TwitchApiResponse, String> {
    client
        .post("https://gql.twitch.tv/gql")
        .body("{\"query\":\"query {\\n  user(login:\\\"alts_alt_\\\") {\\n stream {\\n id\\n}\\n}\\n}\"}")
        .header("Client-Id", "kimne78kx3ncx6brgo4mv6wki5h1ko")
        .send()
        .map_err(|e| e.to_string())
        .and_then(get_is_live_from_response)
        .await
        .map(|is_live| TwitchApiResponse { is_live })
}

async fn twitch(
    State(state): State<TwitchState>,
) -> Result<Json<TwitchApiResponse>, (StatusCode, String)> {
    let is_live = state
        .cache
        .try_get_with(CACHE_KEY, fetch_twitch_api_response(&state.client))
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.as_ref().clone()))?;

    Ok(Json(is_live))
}

pub fn routes() -> Router<TwitchState> {
    Router::new().route("/api/twitch", get(twitch))
}
