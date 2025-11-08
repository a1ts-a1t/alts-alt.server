mod cache;
mod cors;
mod kennel;
mod twitch;

use cache::Cache;
use cors::Cors;
use rocket::fs::{FileServer, NamedFile};
use rocket::futures::{SinkExt, StreamExt};
use rocket::{catch, catchers, get, routes};
use std::path::Path;
use twitch::twitch_handler;
use ws::Message;

use crate::kennel::{init_kennel, kennel_routes, ws_kennel_routes};

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

#[get("/ping")]
fn ws_ping_handler(ws: ws::WebSocket) -> ws::Channel<'static> {
    ws.channel(move |mut stream| {
        Box::pin(async move {
            while let Some(message) = stream.next().await {
                match message {
                    Ok(Message::Close(_)) | Err(_) => break,
                    Ok(Message::Text(_)) => stream.send(Message::text("pong")).await?,
                    _ => {}
                }
            }

            Ok(())
        })
    })
}

#[rocket::main]
async fn main() -> Result<(), String> {
    let (kennel, kennel_cleanup) = init_kennel();
    let _server = rocket::build()
        .mount("/api/kennel-club", kennel_routes())
        .mount("/api", routes![ping_handler, twitch_handler,])
        .mount("/ws/kennel-club", ws_kennel_routes())
        .mount("/ws", routes![ws_ping_handler])
        .mount("/", FileServer::from("./static"))
        .register("/", catchers![not_found])
        .manage(Cache::<String, String>::default())
        .manage(kennel)
        .attach(kennel_cleanup)
        .attach(Cors)
        .launch()
        .await;

    Ok(())
}
