use axum::http::header::{HeaderName, AUTHORIZATION};
use axum::http::HeaderValue;
use axum::response::IntoResponse;
use axum::routing::post;
use axum::Extension;
use axum::{routing::get, Router};
use serde_json::json;
use std::iter::once;
use std::net::SocketAddr;
use tower_http::cors::CorsLayer;
use tower_http::sensitive_headers::SetSensitiveRequestHeadersLayer;
use tower_http::trace::TraceLayer;
use tower_http::{
    compression::CompressionLayer, propagate_header::PropagateHeaderLayer,
};
use tracing::instrument;
use tracing_subscriber::prelude::*;

mod config;
mod db;
mod ffprobe;
mod handlers;
mod media;
pub mod models;
mod oauth;
pub mod schema;
mod serde;
mod state;
mod structs;
mod task;

#[tokio::main]
async fn main() -> Result<(), axum::BoxError> {
    let config = config::load_config().expect("failed to load config");

    let state = state::AppState::new(config);

    let pool = db::create_pool().await;

    run(state, |app| {
        // Define routes for ra-data-simple-rest
        app.nest("/records", {
            Router::new()
                // streams resource
                .route(
                    "/streams",
                    get(handlers::stream::get_list::handler)
                        .post(handlers::stream::create::handler)
                        .put(handlers::stream::create_bulk::handler),
                )
                .route(
                    "/streams/:record_id",
                    get(handlers::stream::get_one::handler)
                        .put(handlers::stream::update::handler)
                        .delete(handlers::stream::delete::handler),
                )
                // video_clips resource
                .route(
                    "/video_clips",
                    get(handlers::video_clip::get_list::handler)
                        .post(handlers::video_clip::create::handler),
                )
                .route(
                    "/video_clips/:record_id",
                    get(handlers::video_clip::get_one::handler)
                        .put(handlers::video_clip::update::handler)
                        .delete(handlers::video_clip::delete::handler),
                )
                // episodes resource
                .route(
                    "/episodes",
                    get(handlers::episode::get_list::handler)
                        .post(handlers::episode::create::handler)
                        .put(handlers::episode::create_bulk::handler),
                )
                .route(
                    "/episodes/:record_id",
                    get(handlers::episode::get_one::handler)
                        .put(handlers::episode::update::handler)
                        .delete(handlers::episode::delete::handler),
                )
                // topics resource
                .route(
                    "/topics",
                    get(handlers::topics::get_list::handler)
                        .post(handlers::topics::create::handler),
                )
                // series resource
                .route(
                    "/series",
                    get(handlers::series::get_list::handler)
                        .post(handlers::series::create::handler),
                )
                .route(
                    "/series/:record_id",
                    get(handlers::series::get_one::handler)
                        .put(handlers::series::update::handler)
                        .delete(handlers::series::delete::handler),
                )
        })
        .route("/chat", post(handlers::chat::handler))
        .route(
            "/stream_ingestion/find_files",
            get(handlers::stream_ingestion::find_files),
        )
        .route(
            "/stream_ingestion/find_rendered_episode_files",
            get(handlers::stream_ingestion::find_rendered_episode_files),
        )
        .route(
            "/tasks",
            get(handlers::tasks::get_list_handler)
                .post(handlers::tasks::create_handler),
        )
        .route(
            "/tasks/:record_id",
            get(handlers::tasks::get_one_handler)
                .put(handlers::tasks::update_handler)
                .delete(handlers::tasks::delete_handler),
        )
        .route("/tasks/ws", get(handlers::tasks::ws_handler))
        .route(
            "/transcription/detect/segment",
            post(handlers::transcription::detect_segment),
        )
        .route(
            "/transcription/detect",
            post(handlers::transcription::detect),
        )
        .route(
            "/silence_detection/detect/segment",
            post(handlers::silence_detection::detect_segment),
        )
        .route(
            "/silence_detection/detect",
            post(handlers::silence_detection::detect),
        )
        .route(
            "/twitch/login",
            get(handlers::twitch::get_login_handler)
                .post(handlers::twitch::post_login_handler),
        )
        .route("/twitch/videos", get(handlers::twitch::list_videos_handler))
        .route(
            "/youtube/login",
            get(handlers::youtube::get_login_handler)
                .post(handlers::youtube::post_login_handler),
        )
        .route(
            "/youtube/upload",
            post(handlers::youtube::upload_start_task_handler),
        )
        .route(
            "/youtube/upload/task",
            post(handlers::youtube::upload_video_handler),
        )
        .route(
            "/youtube/playlist/add/task",
            post(handlers::youtube::add_to_playlist_task_handler),
        )
        .layer(Extension(pool))
    })
    .await
}

async fn run<State>(
    state: State,
    add_routes: impl FnOnce(Router<State>) -> Router<State>,
) -> Result<(), axum::BoxError>
where
    State: Clone + Send + Sync + 'static,
{
    init_tracer();

    // build our application with a route
    let app = app(state, add_routes).await;

    let host: std::net::IpAddr = dotenvy::var("HOST")
        .expect("HOST not set")
        .parse()
        .expect("HOST is not a valid IP address");

    let port = dotenvy::var("PORT")
        .expect("PORT not set")
        .parse()
        .expect("PORT is not a valid port number");

    let addr = SocketAddr::from((host, port));
    tracing::debug!("listening on {}", addr);

    let listener = tokio::net::TcpListener::bind(&addr).await?;

    axum::serve(listener, app.into_make_service())
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

async fn app<State>(
    state: State,
    add_routes: impl FnOnce(Router<State>) -> Router<State>,
) -> Router
where
    State: Clone + Send + Sync + 'static,
{
    let origins = dotenvy::var("CORS_ALLOWED_ORIGINS")
        .expect("CORS_ALLOWED_ORIGINS not set")
        .split(',')
        .map(|s| s.to_string())
        .map(|s| s.parse::<HeaderValue>().unwrap())
        .collect::<Vec<_>>();

    // build our application with a route
    add_routes(Router::<State>::new())
        .route("/health", get(health))
        .with_state(state)
        .layer(CorsLayer::new().allow_origin(origins))
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

#[instrument]
async fn health() -> impl IntoResponse {
    tracing::info!("health check");

    axum::Json(json!({ "status" : "UP" }))
}

async fn shutdown_signal() {
    let ctrl_c = async {
        tokio::signal::ctrl_c()
            .await
            .expect("failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        tokio::signal::unix::signal(
            tokio::signal::unix::SignalKind::terminate(),
        )
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

#[cfg(test)]
mod tests {
    use tokio::runtime::Runtime;

    use super::*;

    #[test]
    fn test_health() {
        let rt = Runtime::new().unwrap();
        let response = rt.block_on(health()).into_response();
        assert_eq!(response.status(), 200);
    }
}
