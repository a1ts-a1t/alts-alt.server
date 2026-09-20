mod cache;
mod kennel;
mod twitch;

use axum::extract::ws::{Message, WebSocketUpgrade};
use axum::response::{IntoResponse, Response};
use axum::routing::{Router, get};
use std::future::IntoFuture;
use std::sync::Arc;
use std::time::Duration;
use tower_http::cors::CorsLayer;
use tower_http::services::{ServeDir, ServeFile};

use crate::cache::Cache;
use crate::kennel::{init_kennel, kennel_routes, ws_kennel_routes};

#[derive(Clone)]
pub struct AppState {
    pub cache: Arc<Cache<String, String>>,
    pub kennel: Arc<kennel::State>,
}

async fn ping_handler() -> &'static str {
    "pong"
}

async fn ws_ping_handler(ws: WebSocketUpgrade) -> Response {
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
    let state = AppState {
        cache: Arc::new(Cache::default()),
        kennel: kennel.clone(),
    };

    let static_files =
        ServeDir::new("./static").fallback(ServeFile::new("./static/not_found.html"));

    let app = Router::new()
        .route("/ws/ping", get(ws_ping_handler))
        .route("/api/ping", get(ping_handler))
        .route("/api/twitch", get(twitch::twitch_handler))
        .merge(kennel_routes())
        .merge(ws_kennel_routes())
        .fallback_service(static_files)
        .layer(CorsLayer::very_permissive())
        .with_state(state);

    let addr = std::env::var("SERVER_ADDRESS").unwrap_or_else(|_| "0.0.0.0:8000".to_string());
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
            Err(_) => {} // 10s timeout ellapsed
        },
    }

    kennel.shutdown().await;
    Ok(())
}
