mod cache;
mod cors;
mod kennel;
mod twitch;

use cache::Cache;
use cors::Cors;
use rocket::fs::{FileServer, NamedFile};
use rocket::{catch, catchers, get, launch, routes};
use std::path::Path;
use twitch::twitch_handler;

use crate::kennel::{
    creature_handler, creature_img_handler, init_kennel, kennel_handler, kennel_img_handler,
};

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
    let (kennel, kennel_cleanup) = init_kennel();
    rocket::build()
        .mount(
            "/api/kennel-club",
            routes![
                kennel_handler,
                kennel_img_handler,
                creature_handler,
                creature_img_handler,
            ],
        )
        .mount("/api", routes![ping_handler, twitch_handler,])
        .mount("/", FileServer::from("./static"))
        .register("/", catchers![not_found])
        .manage(Cache::<String, String>::default())
        .manage(kennel)
        .attach(kennel_cleanup)
        .attach(Cors)
}
