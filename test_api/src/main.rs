use axum::http::header::{HeaderName, AUTHORIZATION};
use axum::{routing::get, routing::post, Router};
use std::iter::once;
use std::net::SocketAddr;
use tower_http::cors::CorsLayer;
use tower_http::sensitive_headers::SetSensitiveRequestHeadersLayer;
use tower_http::trace::TraceLayer;
use tower_http::{compression::CompressionLayer, propagate_header::PropagateHeaderLayer};
use tracing_subscriber::prelude::*;

mod db;
mod handlers;
mod models;
mod schema;
mod state;

#[tokio::main]
async fn main() -> Result<(), axum::BoxError> {
    init_tracer();

    // build our application with a route
    let app = app().await;

    let host: std::net::IpAddr = std::env::var("HOST")
        .expect("HOST not set")
        .parse()
        .expect("HOST is not a valid IP address");
    let port = std::env::var("PORT")
        .expect("PORT not set")
        .parse()
        .expect("PORT is not a valid port number");

    let addr = SocketAddr::from((host, port));
    tracing::debug!("listening on {}", addr);
    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .with_graceful_shutdown(shutdown_signal())
        .await
        .unwrap();

    Ok(())
}

fn init_tracer() {
    let fmt_layer = tracing_subscriber::fmt::layer();

    tracing_subscriber::registry()
        .with(fmt_layer)
        .with(tracing_subscriber::EnvFilter::from_default_env())
        .init();
}

async fn app() -> Router {
    // get path to openai key from env var
    let openai_key_path = std::env::var("OPENAI_KEY_PATH").expect("OPENAI_KEY_PATH not set");

    let state = state::AppState::new(
        std::fs::read_to_string(openai_key_path)
            .expect("failed to read openai key from OPENAI_KEY_PATH")
            .trim()
            .to_string(),
        db::create_pool().await,
    );

    // build our application with a route
    Router::new()
        // `POST /api/chat` goes to `complete_chat`
        .route("/api/chat", post(handlers::complete_chat::handler))
        .route("/", get(handlers::health::handler))
        .with_state(state)
        // TODO make this only allow requests from our frontend?
        .layer(CorsLayer::permissive())
        // Mark the `Authorization` request header as sensitive so it doesn't show in logs
        .layer(SetSensitiveRequestHeadersLayer::new(once(AUTHORIZATION)))
        // High level logging of requests and responses
        .layer(TraceLayer::new_for_http())
        // Compress responses
        .layer(CompressionLayer::new())
        // Propagate `X-Request-Id`s from requests to responses
        .layer(PropagateHeaderLayer::new(HeaderName::from_static(
            "x-request-id",
        )))
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
            .expect("failed to install signal handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {},
        _ = terminate => {},
    }

    tracing::warn!("signal received, starting graceful shutdown");
}
