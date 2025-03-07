use aws_config::{BehaviorVersion, meta::region::RegionProviderChain};
use aws_sdk_secretsmanager::client::Client as SecretsManagerClient;
use axum::{
    Json, Router,
    body::Body,
    http::{Request, StatusCode, header},
    routing::{get, post},
};
use lambda_http::tower;

use serde_json::json;
use std::sync::Arc;
use structs::AppState;
use tower_http::{compression::CompressionLayer, trace::TraceLayer};
use types::utils::YouTubeCredentials;

mod handlers;
mod structs;
mod youtube;

#[tokio::main]
async fn main() {
    // https://docs.aws.amazon.com/lambda/latest/dg/rust-logging.html
    tracing_subscriber::fmt()
        .json()
        // allow log level to be overridden by RUST_LOG env var
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        // this needs to be set to remove duplicated information in the log.
        .with_current_span(false)
        // this needs to be set to false, otherwise ANSI color codes will
        // show up in a confusing manner in CloudWatch logs.
        .with_ansi(false)
        // disabling time is handy because CloudWatch will add the ingestion time.
        .without_time()
        // remove the name of the function from every log entry
        .with_target(false)
        .init();

    let config = structs::load_config().expect("failed to load config");
    let region_provider =
        RegionProviderChain::default_provider().or_else("us-east-1");
    let aws_config = aws_config::defaults(BehaviorVersion::latest())
        .region(region_provider)
        .load()
        .await;

    let secrets_manager = SecretsManagerClient::new(&aws_config);

    let youtube_credentials = match secrets_manager
        .get_secret_value()
        .secret_id(&config.youtube_secret_arn)
        .send()
        .await
    {
        Ok(secret) => match serde_json::from_str::<YouTubeCredentials>(
            secret.secret_string.as_deref().unwrap_or("{}"),
        ) {
            Ok(credentials) => credentials,
            Err(e) => {
                tracing::error!("failed to parse YouTube secret: {:?}", e);
                return;
            }
        },
        Err(e) => {
            tracing::error!("failed to get YouTube secret: {:?}", e);
            return;
        }
    };

    // Create a shared state to pass to the handler
    let state = AppState {
        secrets_manager: Arc::new(secrets_manager),
        youtube_credentials,
        config,
    };

    initialize_api(state).await;
}

async fn initialize_api(state: AppState) {
    // Set up a trace layer
    let trace_layer = TraceLayer::new_for_http().on_request(
        |request: &Request<Body>, _: &tracing::Span| {
            tracing::info!(
                "received request: {method} {uri}",
                method = request.method(),
                uri = request.uri()
            );
        },
    );

    let compression_layer = CompressionLayer::new().gzip(true).deflate(true);

    // Create Axum app
    let app = Router::new()
        .route(
            "/auth/youtube/url",
            post(handlers::obtain_youtube_authorization_url_handler),
        )
        .route(
            "/auth/youtube/callback",
            post(handlers::youtube_callback_handler),
        )
        .route(
            "/auth/youtube/token",
            get(handlers::obtain_youtube_access_token_handler),
        )
        .fallback(|| async {
            (
                StatusCode::NOT_FOUND,
                [(header::CONTENT_TYPE, "application/json")],
                Json(json!({
                    "message": "not found",
                })),
            )
        })
        .layer(trace_layer)
        .layer(compression_layer)
        .with_state(state);

    // Provide the app to the lambda runtime
    let app = tower::ServiceBuilder::new()
        .layer(axum_aws_lambda::LambdaLayer::default().trim_stage())
        .service(app);

    lambda_http::run(app).await.unwrap();
}
