mod cache;
mod twitch;

use std::path::Path;
use rocket::{catch, catchers, get, launch, routes};
use rocket::fs::{FileServer, NamedFile};

#[catch(404)]
async fn not_found() -> Option<NamedFile> {
    NamedFile::open(Path::new("./static/404.html")).await.ok()
}

#[get("/ping")]
fn ping() -> &'static str {
    "pong"
}

#[launch]
fn rocket() -> _ {
    rocket::build()
        .mount("/api", routes![ping])
        .mount("/", FileServer::from("./static"))
        .register("/", catchers![not_found])
}
