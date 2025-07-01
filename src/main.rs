mod cache;
mod twitch;

use cache::Cache;
use rocket::fs::{FileServer, NamedFile};
use rocket::{catch, catchers, get, launch, routes};
use std::path::Path;
use twitch::twitch_handler;

#[catch(404)]
async fn not_found() -> Option<NamedFile> {
    NamedFile::open(Path::new("./static/not_found.html"))
        .await
        .ok()
}

#[get("/ping")]
fn ping_handler() -> &'static str {
    "pong"
}

#[launch]
fn rocket() -> _ {
    rocket::build()
        .mount("/api", routes![ping_handler, twitch_handler])
        .mount("/", FileServer::from("./static"))
        .register("/", catchers![not_found])
        .manage(Cache::<String, String>::default())
}
