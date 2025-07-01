use crate::cache::Cache;
use reqwest::{Response, Client};
use rocket::http;
use rocket::futures::TryFutureExt;
use rocket::{get, State};
use rocket::serde::{Serialize, Deserialize};
use rocket::serde::json::{self, Json};
use rocket::serde::json::serde_json::{self, Value};

const CACHE_KEY: &str = "IS_LIVE_TWITCH_API_CACHE_KEY";

#[derive(Serialize, Deserialize)]
#[serde(crate = "rocket::serde")]
pub struct TwitchApiResponse {
    is_live: bool,
}

async fn get_is_live_from_response(response: Response) -> Result<bool, String> {
    response.text()
        .await
        .map_err(|e| e.to_string())
        .and_then(|body| serde_json::from_str::<Value>(body.as_str()).map_err(|e| e.to_string()))
        .and_then(|json_body| json_body.pointer("/data/user/stream").cloned().ok_or("Unable to determine `is_live` status.".to_string()))
        .map(|val| !val.is_null())
}

async fn fetch_twitch_api_response() -> Result<TwitchApiResponse, String> {
    Client::new()
        .post("https://gql.twitch.tv/gql")
        .body("{\"query\":\"query {\\n  user(login:\\\"alts_alt\\\") {\\n stream {\\n id\\n}\\n}\\n}\"}")
        .header("Client-Id", "kimne78kx3ncx6brgo4mv6wki5h1ko")
        .send()
        .map_err(|e| e.to_string())
        .and_then(get_is_live_from_response)
        .await
        .map(|is_live| TwitchApiResponse { is_live })
}

#[get("/twitch")]
pub async fn twitch_handler(cache: &State<Cache<String, String>>) -> Result<Json<TwitchApiResponse>, (http::Status, String)> {
    let cache_value = cache.get(&CACHE_KEY.to_string())
        .ok_or(())
        .and_then(|val| json::from_str::<TwitchApiResponse>(&val).map_err(|_| ()));

    match cache_value {
        Ok(val) => Ok(Json(val)),
        Err(_) => {
            let res = fetch_twitch_api_response().await;
            res.inspect(|val| (cache.put(CACHE_KEY.to_string(), serde_json::to_string(val).expect("Unable to deserialize Twitch API response."))))
                .map(|val| Json(val))
                .map_err(|e| (http::Status::InternalServerError, e))
        },
    }
}

