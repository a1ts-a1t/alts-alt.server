use std::time::Duration;

use serde_json::Value;
use futures_util::FutureExt;
use http::{Response, StatusCode};

use crate::{cache::{create_cached_async_fn, create_cached_fn, CacheConfig}, router::Router, types::{RouterFuture, RouterRequest}, utils::create_response_body_from_string};

fn ping_route_fn(_: RouterRequest) -> RouterFuture {
    let body = create_response_body_from_string("pong".to_string());
    let res = Response::new(body);
    Box::pin(async { Ok(res) })
}

fn create_is_live_route_fn() -> impl Fn(RouterRequest) -> RouterFuture + Clone + Send + Sync + 'static {
    let fetch_is_live = async || {
        println!("Fetching!!!");
        let client = reqwest::Client::new();
        let response_result = client.post("https://gql.twitch.tv/gql")
            .header("Client-Id", "kimne78kx3ncx6brgo4mv6wki5h1ko")
            .body(r#"{"query":"query { user(login:\"alts_alt_\") { stream { id } } }"}"#)
            .send()
            .await;

        let text = match response_result {
            Ok(response) => response.text().await.ok(),
            Err(_) => None,
        };
            
        text.and_then(|text| serde_json::from_str::<Value>(&text).ok())
            .and_then(|json| {
                json.pointer("/data/user/stream/id")
                    .and_then(|value| value.as_str())
                    .map(|str| str.to_string())
            })
            .is_some()
    };

    let config = CacheConfig {
        max_age: Duration::new(10, 0),
    };
    let cached_fetch_is_live = create_cached_async_fn(fetch_is_live, config);

    move |_| {
        let fut = cached_fetch_is_live().then(async move |is_live| {
            let mut response = Response::new(create_response_body_from_string(is_live.to_string()));
            *response.status_mut() = StatusCode::OK;
            response
        });
        Box::pin(async { Ok(fut.await) })
    }
}

pub(crate) fn get_api_router() -> Router {
    let mut router = Router::new();

    router.with_route_fn(vec!("ping".to_string()), ping_route_fn);
    router.with_route_fn(vec!("is-live".to_string()), create_is_live_route_fn());

    router
}

