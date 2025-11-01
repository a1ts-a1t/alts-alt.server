use std::{path::PathBuf, sync::Arc};

use kennel_club::ImageFormat;
use rocket::{
    Route, State as RocketState,
    fairing::AdHoc,
    get,
    http::{self},
    routes,
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
async fn kennel_handler(kennel: &RocketState<Arc<State>>) -> Response {
    Response::new_json(kennel.as_json())
}

#[get("/img")]
async fn kennel_img_handler(kennel: &RocketState<Arc<State>>) -> Response {
    match kennel.as_image(ImageFormat::Png) {
        Ok(data) => Response::new_image(data, ImageFormat::Png),
        Err(message) => Response::new_err(http::Status::InternalServerError, &message),
    }
}

#[get("/<creature_id>")]
async fn creature_handler(creature_id: &str, kennel: &RocketState<Arc<State>>) -> Response {
    match kennel.get_creature(creature_id) {
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
    match kennel.get_creature(creature_id) {
        Some(creature) => Response::new_permanent_redirect(creature.url()),
        None => Response::new_err(
            http::Status::NotFound,
            &format!("{} not found", creature_id),
        ),
    }
}

#[get("/random")]
async fn random_creature_handler(kennel: &RocketState<Arc<State>>) -> Response {
    match kennel.get_random_creature() {
        Some(creature) => Response::new_json(creature),
        None => Response::new_err(http::Status::NotFound, "No creatures found"),
    }
}

#[get("/random/site")]
async fn random_creature_site_handler(kennel: &RocketState<Arc<State>>) -> Response {
    match kennel.get_random_creature() {
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
