use serde_json::Value;
use futures_util::FutureExt;
use http::{Response, StatusCode};

use crate::{router::Router, types::{RouterFuture, RouterRequest}, utils::create_response_body_from_string};

fn ping_route_fn(_: RouterRequest) -> RouterFuture {
    let body = create_response_body_from_string("pong".to_string());
    let res = Response::new(body);
    Box::pin(async { Ok(res) })
}

fn is_live_route_fn(_: RouterRequest) -> RouterFuture {
    let client = reqwest::Client::new();
    let res_fut = client.post("https://gql.twitch.tv/gql")
        .header("Client-Id", "kimne78kx3ncx6brgo4mv6wki5h1ko")
        .body(r#"{"query":"query { user(login:\"alts_alt\") { stream { id } } }"}"#)
        .send();

    let is_live_fut = res_fut.then(|response_result| async move {
        let text = match response_result {
            Ok(response) => response.text().await.ok(),
            Err(_) => None,
        };
        
        let is_live = text
            .and_then(|text| serde_json::from_str::<Value>(&text).ok())
            .and_then(|json| {
                json.pointer("/data/user/stream")
                    .and_then(|value| value.as_bool())
            })
            .unwrap_or(false);

        is_live
    });

    let response_fut = is_live_fut.then(async move |is_live| {
        let mut response = Response::new(create_response_body_from_string(is_live.to_string()));
        *response.status_mut() = StatusCode::OK;
        response
    });

    Box::pin(async { Ok(response_fut.await) })
}

pub(crate) fn get_api_router() -> Router {
    let mut router = Router::new();

    router.with_route_fn(vec!("ping".to_string()), ping_route_fn);
    router.with_route_fn(vec!("is-live".to_string()), is_live_route_fn);

    router
}

