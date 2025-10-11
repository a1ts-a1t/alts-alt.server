use std::{path::PathBuf, sync::Arc};

pub use state::State;
use rocket::{fairing::AdHoc, get, http, serde::json::Json, State as RocketState};

use crate::kennel::json::CreatureJson;

mod state;
mod json;

pub fn init_kennel() -> (Arc<State>, AdHoc) {
    let dir = PathBuf::from("./kennel-club");
    let kennel = State::load(&dir).expect("Error loading kennel");
    let kennel = Arc::new(kennel);

    let kennel_clone = kennel.clone();
    let cleanup = AdHoc::on_shutdown(
        "Kennel shutdown",
        |_| Box::pin(async move {
            kennel_clone.shutdown();
        })
    );

    (kennel, cleanup)
}

#[get("/kennel-club")]
pub async fn kennel_handler(
    kennel: &RocketState<Arc<State>>,
) -> Result<Json<Vec<CreatureJson>>, (http::Status, String)> {
    kennel.as_json()
        .map(|c| Json(c))
        .map_err(|s| (http::Status::InternalServerError, s.to_string()))
}
