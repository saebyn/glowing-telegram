use axum::http::header::{HeaderName, AUTHORIZATION};
use axum::{routing::get, routing::post, Router};
use std::iter::once;
use std::net::SocketAddr;
use tower_http::cors::CorsLayer;
use tower_http::sensitive_headers::SetSensitiveRequestHeadersLayer;
use tower_http::trace::TraceLayer;
use tower_http::{compression::CompressionLayer, propagate_header::PropagateHeaderLayer};

mod db;
mod handlers;
mod models;
mod schema;
mod state;

#[tokio::main]
async fn main() -> Result<(), axum::BoxError> {
    // initialize tracing
    tracing_subscriber::fmt::init();

    // build our application with a route
    let app = app().await;

    // run our app with hyper
    let addr = SocketAddr::from(([127, 0, 0, 1], 3000));
    tracing::debug!("listening on {}", addr);
    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .with_graceful_shutdown(shutdown_signal())
        .await
        .unwrap();

    Ok(())
}

async fn app() -> Router {
    let state = state::AppState::new(
        std::fs::read_to_string("../openai_key.txt")
            .expect("failed to read openai_key.txt")
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
