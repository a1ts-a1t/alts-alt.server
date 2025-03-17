mod types;
mod router;
mod static_server;
mod utils;

use http::{Response, StatusCode};
use hyper::server::conn::http1;
use hyper_util::rt::TokioIo;
use router::Router;
use static_server::StaticServer;
use types::{RouterFuture, RouterRequest};
use utils::create_response_body_from_string;
use std::net::SocketAddr;
use tokio::net::TcpListener;

const PORT_ENV_VAR: &str = "PORT";
const ROOT_ENV_VAR: &str = "STATIC_SERVER_ROOT";
const FALLBACK_FILE_ENV_VAR: &str = "STATIC_SERVER_FALLBACK_FILE";

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
    // initialize environment
    let port: u16 = match std::env::var(PORT_ENV_VAR) {
        Ok(env_var) => env_var.parse::<u16>().unwrap_or(8080),
        Err(_) => 8080,
    };

    let static_server_root: String = match std::env::var(ROOT_ENV_VAR) {
        Ok(env_var) => env_var,
        Err(_) => "build".to_string(),
    };

    let static_server_fallback_file: String = match std::env::var(FALLBACK_FILE_ENV_VAR) {
        Ok(env_var) => env_var,
        Err(_) => "build/404.html".to_string(),
    };

    // initialize service components
    let mut static_server = StaticServer::new();
    static_server.with_root(static_server_root);
    static_server.with_fallback_file(static_server_fallback_file);

	let mut router = Router::new();
	router.with_route_fn(vec!("api".to_string(), "ping".to_string()), ping_endpoint);
    router.with_service(vec!(), static_server);

    let address = SocketAddr::from(([127, 0, 0, 1], port));
    let listener = TcpListener::bind(address).await?;


    // listen for requests
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

