mod types;
mod router;
mod static_server;
mod utils;

use crate::router::Router;

use http::{Response, StatusCode};
use hyper::server::conn::http1;
use hyper_util::rt::TokioIo;
use static_server::StaticServer;
use types::{RouterFuture, RouterRequest};
use utils::create_response_body_from_string;
use std::net::SocketAddr;
use tokio::net::TcpListener;

fn ping_endpoint(_: RouterRequest) -> RouterFuture {
	let body = create_response_body_from_string("pong".to_string());
	let res = Response::builder()
		.status(StatusCode::OK)
		.body(body)
		.unwrap();
	return Box::pin(async { Ok(res) });
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let address = SocketAddr::from(([127, 0, 0, 1], 8080));
    let listener = TcpListener::bind(address).await?;

    let mut static_server = StaticServer::new();
    static_server.with_root("build".to_string());
    static_server.with_fallback_file("build/test2.txt".to_string());

	let mut router = Router::new();
	router.with_route_fn(vec!("ping".to_string()), ping_endpoint);
    router.with_service(vec!(), static_server);

    loop {
        let (stream, _) = listener.accept().await?;
        let io = TokioIo::new(stream);
		let router_clone = router.clone();

        tokio::task::spawn(async move {
            if let Err(err) = http1::Builder::new().serve_connection(io, router_clone).await {
                eprintln!("Error sending connection: {}", err);
            }
        });
    }
}

