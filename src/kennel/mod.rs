use std::{path::PathBuf, sync::Arc};

use kennel_club::ImageFormat;
use rocket::{
    State as RocketState,
    fairing::AdHoc,
    get,
    http::{self, Accept, uncased::UncasedStr},
};
pub use state::State;

use crate::kennel::response::Response;

mod json;
mod response;
mod state;

pub fn init_kennel() -> (Arc<State>, AdHoc) {
    let dir = PathBuf::from("./kennel-club");
    let kennel = State::load(&dir).expect("Error loading kennel");
    let kennel = Arc::new(kennel);

    let kennel_clone = kennel.clone();
    let cleanup = AdHoc::on_shutdown("Kennel shutdown", |_| {
        Box::pin(async move {
            kennel_clone.shutdown();
        })
    });

    (kennel, cleanup)
}

#[get("/")]
pub async fn kennel_handler(kennel: &RocketState<Arc<State>>) -> Response {
    Response::new_json(kennel.as_json())
}

#[get("/img")]
pub async fn kennel_img_handler(accept: &Accept, kennel: &RocketState<Arc<State>>) -> Response {
    let media_type = accept.preferred().media_type();
    let image_format = media_type
        .extension()
        .map(UncasedStr::as_str)
        .and_then(ImageFormat::from_extension)
        .unwrap_or(ImageFormat::Png);

    match kennel.as_image(image_format) {
        Ok(data) => Response::new_image(data, image_format),
        Err(message) => Response::new_err(http::Status::InternalServerError, &message),
    }
}

#[get("/<creature_id>")]
pub async fn creature_handler(creature_id: &str, kennel: &RocketState<Arc<State>>) -> Response {
    match kennel.get_creature(creature_id) {
        Some(creature) => Response::new_json(creature),
        None => Response::new_err(
            http::Status::NotFound,
            &format!("{} not found", creature_id),
        ),
    }
}

#[get("/<creature_id>/img")]
pub async fn creature_img_handler(creature_id: &str, kennel: &RocketState<Arc<State>>) -> Response {
    let (bytes, format) = kennel
        .get_sprite(creature_id)
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
