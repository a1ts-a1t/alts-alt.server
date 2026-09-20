use std::{path::PathBuf, sync::Arc};

use axum::extract::ws::{Message, WebSocketUpgrade};
use axum::extract::{Path, State as AxumState};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response as AxumResponse};
use axum::routing::{Router, get};
use futures_util::{SinkExt, StreamExt};
use kennel_club::ImageFormat;
use tokio_stream::wrappers::ReceiverStream;

use crate::AppState;
use crate::kennel::{response::Response, stream::greedy_zip};

mod json;
mod response;
mod state;
mod stream;

pub use state::State;

pub fn init_kennel() -> Arc<State> {
    let dir = PathBuf::from("./kennel-club");
    Arc::new(State::load(&dir).expect("Error loading kennel"))
}

async fn kennel_handler(AxumState(state): AxumState<AppState>) -> Response {
    Response::new_json(state.kennel.as_json().await)
}

async fn kennel_img_handler(AxumState(state): AxumState<AppState>) -> Response {
    match state.kennel.as_image(ImageFormat::Png).await {
        Ok(data) => Response::new_image(data, ImageFormat::Png),
        Err(message) => Response::new_err(StatusCode::INTERNAL_SERVER_ERROR, &message),
    }
}

async fn creature_handler(
    Path(creature_id): Path<String>,
    AxumState(state): AxumState<AppState>,
) -> Response {
    match state.kennel.get_creature(&creature_id).await {
        Some(creature) => Response::new_json(creature),
        None => Response::new_err(StatusCode::NOT_FOUND, &format!("{} not found", creature_id)),
    }
}

async fn creature_img_handler(
    Path(creature_id): Path<String>,
    AxumState(state): AxumState<AppState>,
) -> Response {
    let (bytes, format) = state
        .kennel
        .get_sprite(&creature_id)
        .await
        .map(|sprite| (sprite.bytes(), sprite.format()))
        .unzip();

    match (bytes, format) {
        (Some(b), Some(f)) => Response::new_image(b, f),
        _ => Response::new_err(StatusCode::NOT_FOUND, &format!("{} not found", creature_id)),
    }
}

async fn creature_img_by_handler(
    Path((creature_id, sprite_state, frame)): Path<(String, String, usize)>,
    AxumState(state): AxumState<AppState>,
) -> Response {
    let (bytes, format) = state
        .kennel
        .get_sprite_by(&creature_id, &sprite_state, &frame)
        .await
        .map(|sprite| (sprite.bytes(), sprite.format()))
        .unzip();

    match (bytes, format) {
        (Some(b), Some(f)) => Response::new_cached_image(b, f),
        _ => Response::new_err(StatusCode::NOT_FOUND, &format!("{} not found", creature_id)),
    }
}

async fn creature_site_handler(
    Path(creature_id): Path<String>,
    AxumState(state): AxumState<AppState>,
) -> Response {
    match state.kennel.get_creature(&creature_id).await {
        Some(creature) => Response::new_permanent_redirect(creature.url()),
        None => Response::new_err(StatusCode::NOT_FOUND, &format!("{} not found", creature_id)),
    }
}

async fn random_creature_handler(AxumState(state): AxumState<AppState>) -> Response {
    match state.kennel.get_random_creature().await {
        Some(creature) => Response::new_json(creature),
        None => Response::new_err(StatusCode::NOT_FOUND, "No creatures found"),
    }
}

async fn random_creature_site_handler(AxumState(state): AxumState<AppState>) -> Response {
    match state.kennel.get_random_creature().await {
        Some(creature) => Response::new_temporary_redirect(creature.url()),
        None => Response::new_err(StatusCode::NOT_FOUND, "No creatures found"),
    }
}

pub fn kennel_routes() -> Router<crate::AppState> {
    Router::new()
        .route("/api/kennel-club", get(kennel_handler))
        .route("/api/kennel-club/img", get(kennel_img_handler))
        .route("/api/kennel-club/random", get(random_creature_handler))
        .route(
            "/api/kennel-club/random/site",
            get(random_creature_site_handler),
        )
        .route("/api/kennel-club/{creature_id}", get(creature_handler))
        .route(
            "/api/kennel-club/{creature_id}/img",
            get(creature_img_handler),
        )
        .route(
            "/api/kennel-club/{creature_id}/img/{sprite_state}/{frame}",
            get(creature_img_by_handler),
        )
        .route(
            "/api/kennel-club/{creature_id}/site",
            get(creature_site_handler),
        )
}

async fn ws_kennel_handler(
    ws: WebSocketUpgrade,
    AxumState(state): AxumState<AppState>,
) -> AxumResponse {
    let kennel = state.kennel.clone();
    ws.on_upgrade(move |socket| async move {
        let (mut sender, receiver) = socket.split();
        let (uuid, broadcast) = kennel.subscribe().await;
        let mut stream = greedy_zip(receiver, ReceiverStream::new(broadcast));

        while let Some((message, kennel_json)) = stream.next().await {
            match (message, kennel_json) {
                (Some(Ok(Message::Close(_))), _) | (Some(Err(_)), _) => break,
                (_, Some(json)) => {
                    if let Ok(json_str) = serde_json::to_string(&json)
                        && sender.send(Message::text(json_str)).await.is_err()
                    {
                        break;
                    }
                }
                (_, _) => {}
            }
        }

        kennel.unsubscribe(&uuid).await;
    })
    .into_response()
}

pub fn ws_kennel_routes() -> Router<crate::AppState> {
    Router::new().route("/ws/kennel-club", get(ws_kennel_handler))
}
