use std::{path::PathBuf, sync::Arc};

use kennel_club::ImageFormat;
use rocket::{
    Route, State as RocketState,
    fairing::AdHoc,
    futures::{SinkExt, StreamExt},
    get,
    http::{self},
    routes,
};
pub use state::State;
use tokio_stream::wrappers::ReceiverStream;
use ws::{Message, WebSocket};

use crate::kennel::{response::Response, stream::greedy_zip};

mod json;
mod response;
mod state;
mod stream;

pub fn init_kennel() -> (Arc<State>, AdHoc) {
    let dir = PathBuf::from("./kennel-club");
    let kennel = State::load(&dir).expect("Error loading kennel");
    let kennel = Arc::new(kennel);

    let kennel_clone = kennel.clone();
    let cleanup = AdHoc::on_shutdown("Kennel shutdown", |_| {
        Box::pin(async move {
            kennel_clone.shutdown().await;
        })
    });

    (kennel, cleanup)
}

#[get("/")]
async fn kennel_handler(kennel: &RocketState<Arc<State>>) -> Response {
    Response::new_json(kennel.as_json().await)
}

#[get("/img")]
async fn kennel_img_handler(kennel: &RocketState<Arc<State>>) -> Response {
    match kennel.as_image(ImageFormat::Png).await {
        Ok(data) => Response::new_image(data, ImageFormat::Png),
        Err(message) => Response::new_err(http::Status::InternalServerError, &message),
    }
}

#[get("/<creature_id>")]
async fn creature_handler(creature_id: &str, kennel: &RocketState<Arc<State>>) -> Response {
    match kennel.get_creature(creature_id).await {
        Some(creature) => Response::new_json(creature),
        None => Response::new_err(
            http::Status::NotFound,
            &format!("{} not found", creature_id),
        ),
    }
}

#[get("/<creature_id>/img")]
async fn creature_img_handler(creature_id: &str, kennel: &RocketState<Arc<State>>) -> Response {
    let (bytes, format) = kennel
        .get_sprite(creature_id)
        .await
        .map(|sprite| (sprite.bytes(), sprite.format()))
        .unzip();

    match (bytes, format) {
        (Some(b), Some(f)) => Response::new_image(b, f),
        _ => Response::new_err(
            http::Status::NotFound,
            &format!("{} not found", creature_id),
        ),
    }
}

#[get("/<creature_id>/site")]
async fn creature_site_handler(creature_id: &str, kennel: &RocketState<Arc<State>>) -> Response {
    match kennel.get_creature(creature_id).await {
        Some(creature) => Response::new_permanent_redirect(creature.url()),
        None => Response::new_err(
            http::Status::NotFound,
            &format!("{} not found", creature_id),
        ),
    }
}

#[get("/random")]
async fn random_creature_handler(kennel: &RocketState<Arc<State>>) -> Response {
    match kennel.get_random_creature().await {
        Some(creature) => Response::new_json(creature),
        None => Response::new_err(http::Status::NotFound, "No creatures found"),
    }
}

#[get("/random/site")]
async fn random_creature_site_handler(kennel: &RocketState<Arc<State>>) -> Response {
    match kennel.get_random_creature().await {
        Some(creature) => Response::new_temporary_redirect(creature.url()),
        None => Response::new_err(http::Status::NotFound, "No creatures found"),
    }
}

pub fn kennel_routes() -> Vec<Route> {
    routes![
        kennel_handler,
        kennel_img_handler,
        creature_handler,
        creature_img_handler,
        creature_site_handler,
        random_creature_handler,
        random_creature_site_handler,
    ]
}

#[get("/")]
fn ws_kennel_handler(ws: WebSocket, kennel: &RocketState<Arc<State>>) -> ws::Channel<'static> {
    let kennel_state = kennel.inner().clone();
    ws.channel(move |mut message_stream| {
        Box::pin(async move {
            let (uuid, receiver) = kennel_state.subscribe().await;
            let mut stream = greedy_zip(message_stream.by_ref(), ReceiverStream::new(receiver));

            while let Some((message, kennel_json)) = stream.next().await {
                match (message, kennel_json) {
                    (Some(Ok(Message::Close(_))), _) | (Some(Err(_)), _) => break,
                    (_, Some(json)) => {
                        let (sender, _) = stream.get_mut();
                        if let Ok(json_str) = serde_json::to_string(&json) {
                            sender.send(Message::text(json_str)).await.unwrap();
                        }
                    }
                    (_, _) => {}
                };
            }

            let (_, receiver_stream) = stream.get_mut();
            kennel_state.unsubscribe(&uuid).await;
            receiver_stream.close();

            Ok(())
        })
    })
}

pub fn ws_kennel_routes() -> Vec<Route> {
    routes![ws_kennel_handler,]
}
