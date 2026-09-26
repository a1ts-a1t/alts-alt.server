use std::{path::PathBuf, sync::Arc};

use axum::Json;
use axum::extract::ws::{Message, WebSocketUpgrade};
use axum::extract::{Path, State};
use axum::http::{StatusCode, header};
use axum::response::{IntoResponse, Response};
use axum::routing::{Router, get};
use futures_util::{SinkExt, StreamExt};
use kennel_club::ImageFormat;
use tokio_stream::wrappers::ReceiverStream;

use crate::kennel::stream::greedy_zip;

mod json;
mod state;
mod stream;

pub use state::KennelState;

const NO_CACHE: &str = "no-cache, no-store";
const SPRITE_CACHE: &str = "public, max-age=3600";

enum Error {
    Internal(String),
    NotFound(String),
}

impl From<String> for Error {
    fn from(message: String) -> Self {
        Error::Internal(message)
    }
}

impl IntoResponse for Error {
    fn into_response(self) -> Response {
        match self {
            Error::Internal(message) => (StatusCode::INTERNAL_SERVER_ERROR, message),
            Error::NotFound(message) => (StatusCode::NOT_FOUND, message),
        }
        .into_response()
    }
}

pub fn init_kennel() -> Arc<KennelState> {
    let dir = PathBuf::from("./kennel-club");
    Arc::new(KennelState::load(&dir))
}

async fn kennel(State(kennel): State<Arc<KennelState>>) -> Result<Response, Error> {
    let creatures = kennel.as_json().await?;

    Ok(([(header::CACHE_CONTROL, NO_CACHE)], Json(creatures)).into_response())
}

async fn kennel_img(State(kennel): State<Arc<KennelState>>) -> Result<Response, Error> {
    let data = kennel.as_image(ImageFormat::Png).await?;

    Ok((
        [
            (header::CONTENT_TYPE, ImageFormat::Png.to_mime_type()),
            (header::CACHE_CONTROL, NO_CACHE),
        ],
        data,
    )
        .into_response())
}

async fn creature(
    Path(creature_id): Path<String>,
    State(kennel): State<Arc<KennelState>>,
) -> Result<Response, Error> {
    let Some(creature) = kennel.get_creature(&creature_id).await? else {
        return Err(Error::NotFound(format!("{creature_id} not found")));
    };

    Ok(([(header::CACHE_CONTROL, NO_CACHE)], Json(creature)).into_response())
}

async fn creature_img(
    Path(creature_id): Path<String>,
    State(kennel): State<Arc<KennelState>>,
) -> Result<Response, Error> {
    let Some(sprite) = kennel.get_sprite(&creature_id).await? else {
        return Err(Error::NotFound(format!("{creature_id} not found")));
    };

    Ok((
        [
            (header::CONTENT_TYPE, sprite.format().to_mime_type()),
            (header::CACHE_CONTROL, NO_CACHE),
        ],
        sprite.bytes(),
    )
        .into_response())
}

async fn creature_img_by(
    Path((creature_id, sprite_state, frame)): Path<(String, String, usize)>,
    State(kennel): State<Arc<KennelState>>,
) -> Result<Response, Error> {
    let Some(sprite) = kennel
        .get_sprite_by(&creature_id, &sprite_state, &frame)
        .await?
    else {
        return Err(Error::NotFound(format!("{creature_id} not found")));
    };

    Ok((
        [
            (header::CONTENT_TYPE, sprite.format().to_mime_type()),
            (header::CACHE_CONTROL, SPRITE_CACHE),
        ],
        sprite.bytes(),
    )
        .into_response())
}

async fn creature_site(
    Path(creature_id): Path<String>,
    State(kennel): State<Arc<KennelState>>,
) -> Result<Response, Error> {
    let Some(creature) = kennel.get_creature(&creature_id).await? else {
        return Err(Error::NotFound(format!("{creature_id} not found")));
    };

    Ok((
        StatusCode::MOVED_PERMANENTLY,
        [(header::LOCATION, creature.url())],
    )
        .into_response())
}

async fn random_creature(State(kennel): State<Arc<KennelState>>) -> Result<Response, Error> {
    let Some(creature) = kennel.get_random_creature().await? else {
        return Err(Error::NotFound("No creatures found".to_string()));
    };

    Ok(([(header::CACHE_CONTROL, NO_CACHE)], Json(creature)).into_response())
}

async fn random_creature_site(State(kennel): State<Arc<KennelState>>) -> Result<Response, Error> {
    let Some(creature) = kennel.get_random_creature().await? else {
        return Err(Error::NotFound("No creatures found".to_string()));
    };

    Ok((StatusCode::FOUND, [(header::LOCATION, creature.url())]).into_response())
}

async fn ws_kennel(
    ws: WebSocketUpgrade,
    State(kennel): State<Arc<KennelState>>,
) -> Result<Response, Error> {
    kennel.availability()?;

    Ok(ws
        .on_upgrade(move |socket| async move {
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
        .into_response())
}

pub fn routes() -> Router<Arc<KennelState>> {
    Router::new()
        .route("/api/kennel-club", get(kennel))
        .route("/api/kennel-club/img", get(kennel_img))
        .route("/api/kennel-club/random", get(random_creature))
        .route("/api/kennel-club/random/site", get(random_creature_site))
        .route("/api/kennel-club/{creature_id}", get(creature))
        .route("/api/kennel-club/{creature_id}/img", get(creature_img))
        .route(
            "/api/kennel-club/{creature_id}/img/{sprite_state}/{frame}",
            get(creature_img_by),
        )
        .route("/api/kennel-club/{creature_id}/site", get(creature_site))
        .route("/ws/kennel-club", get(ws_kennel))
}
