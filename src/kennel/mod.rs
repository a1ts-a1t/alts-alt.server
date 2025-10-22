use std::{path::PathBuf, sync::Arc};

use kennel_club::ImageFormat;
use rocket::{
    State as RocketState,
    fairing::AdHoc,
    get,
    http::Accept,
    serde::json::Json,
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

#[get("/kennel-club")]
pub async fn kennel_handler(accept: &Accept, kennel: &RocketState<Arc<State>>) -> Response {
    let media_type = accept.preferred().media_type();

    if media_type.is_png() {
        return match kennel.as_image(ImageFormat::Png) {
            Ok(image) => Response::from_png(image),
            Err(message) => Response::new_err(message),
        };
    }

    if media_type.is_jpeg() {
        return match kennel.as_image(ImageFormat::Jpeg) {
            Ok(image) => Response::from_jpeg(image),
            Err(message) => Response::new_err(message),
        };
    }

    if media_type.is_gif() {
        return match kennel.as_image(ImageFormat::Gif) {
            Ok(image) => Response::from_gif(image),
            Err(message) => Response::new_err(message),
        };
    }

    if media_type.is_webp() {
        return match kennel.as_image(ImageFormat::WebP) {
            Ok(image) => Response::from_webp(image),
            Err(message) => Response::new_err(message),
        };
    }

    // default images to png
    if media_type.top() == "image" {
        return match kennel.as_image(ImageFormat::Png) {
            Ok(image) => Response::from_png(image),
            Err(message) => Response::new_err(message),
        };
    }

    // default to returning json
    match kennel.as_json() {
        Ok(creatures) => Response::from_json(Json(creatures)),
        Err(message) => Response::new_err(message.to_string()),
    }
}
