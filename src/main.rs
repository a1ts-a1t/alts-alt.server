mod cache;
mod kennel;
mod reverse_proxy;
mod twitch;

use std::future::IntoFuture;
use std::time::Duration;

use axum::extract::ws::{Message, WebSocketUpgrade};
use axum::response::{IntoResponse, Response};
use axum::routing::{Router, get};
use tower_http::cors::CorsLayer;

use crate::kennel::init_kennel;
use crate::reverse_proxy::{ReverseProxyConfig, reverse_proxy};

async fn ping() -> &'static str {
    "pong"
}

async fn ws_ping(ws: WebSocketUpgrade) -> Response {
    ws.on_upgrade(|mut socket| async move {
        while let Some(message) = socket.recv().await {
            if message.is_err() || matches!(message, Ok(Message::Close(_))) {
                break;
            }
            if matches!(message, Ok(Message::Text(_)))
                && socket.send(Message::text("pong")).await.is_err()
            {
                break;
            }
        }
    })
    .into_response()
}

async fn shutdown_signal() {
    let ctrl_c = async {
        tokio::signal::ctrl_c()
            .await
            .expect("failed to install Ctrl+C handler");
    };
    #[cfg(unix)]
    let terminate = async {
        tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
            .expect("failed to install SIGTERM handler")
            .recv()
            .await;
    };
    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();
    tokio::select! {
        _ = ctrl_c => {},
        _ = terminate => {},
    }
}

#[tokio::main]
async fn main() -> Result<(), String> {
    let kennel = init_kennel();

    let website_origin =
        std::env::var("WEBSITE_ORIGIN").unwrap_or_else(|_| "http://0.0.0.0:8080".to_string());

    let proxy = Router::new()
        .fallback(reverse_proxy)
        .with_state(ReverseProxyConfig::new(website_origin));

    let app = Router::new()
        .route("/ws/ping", get(ws_ping))
        .route("/api/ping", get(ping))
        .merge(twitch::routes().with_state(twitch::TwitchState::new()))
        .merge(kennel::routes().with_state(kennel.clone()))
        .merge(proxy)
        .layer(CorsLayer::very_permissive());

    let port = std::env::var("SERVER_PORT").unwrap_or_else(|_| "8000".to_string());
    let addr = format!("0.0.0.0:{port}");
    let listener = tokio::net::TcpListener::bind(&addr)
        .await
        .map_err(|e| e.to_string())?;

    let mut serve = axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .into_future();

    // force ungraceful shutdown on 10 second timeout of a shutdown signal
    // or else WS connections will keep it open
    tokio::select! {
        res = &mut serve => match res {
            Ok(()) => {}
            Err(e) => return Err(e.to_string()),
        },
        _ = shutdown_signal() => match tokio::time::timeout(Duration::from_secs(10), &mut serve).await {
            Ok(Ok(())) => {}
            Ok(Err(e)) => return Err(e.to_string()),
            Err(_) => {} // 10s timeout elapsed
        },
    }

    kennel.shutdown().await;
    Ok(())
}
