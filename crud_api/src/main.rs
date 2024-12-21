/**
 * This is the main entrypoint for the `crud_api` lambda function.
 *
 * The function is responsible for handling the requests and responses for the
 * CRUD operations, in a way compatible with the ra-data-simple-rest data
 * provider for React Admin.
 *
 */
use aws_config::{meta::region::RegionProviderChain, BehaviorVersion};
use aws_sdk_dynamodb::Client;
use axum::{
    body::Body,
    extract::{Path, Query, State},
    http::{
        header::{
            self, ACCEPT, ACCEPT_ENCODING, AUTHORIZATION, CONTENT_TYPE, ORIGIN,
        },
        Request, StatusCode,
    },
    response::IntoResponse,
    routing::get,
    Json, Router,
};
use figment::Figment;
use lambda_http::tower;
use serde::Deserialize;
use serde_json::json;
use std::{collections::HashMap, sync::Arc};
use tower_http::{
    compression::CompressionLayer, cors::CorsLayer, trace::TraceLayer,
};

mod dynamodb;

#[derive(Debug, Deserialize, Clone)]
#[allow(clippy::struct_field_names)]
struct Config {
    video_metadata_table: String,
    episodes_table: String,
    streams_table: String,
    series_table: String,
    profiles_table: String,
}

fn load_config() -> Result<Config, figment::Error> {
    let figment = Figment::new().merge(figment::providers::Env::raw());

    figment.extract()
}

#[derive(Debug, Clone)]
struct AppState {
    dynamodb: Arc<Client>,
    config: Config,
}

#[derive(Debug, Deserialize)]
struct RequestPath {
    stage: String,
    resource: String,
}

#[derive(Debug, Deserialize)]
struct RequestPathWithId {
    stage: String,
    resource: String,
    record_id: String,
}

#[derive(Deserialize)]
struct ManyQuery {
    id: Vec<String>,
}

#[tokio::main]
async fn main() {
    // https://docs.aws.amazon.com/lambda/latest/dg/rust-logging.html
    tracing_subscriber::fmt()
        .json()
        .with_max_level(tracing::Level::INFO)
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

    let config = load_config().expect("failed to load config");
    let region_provider =
        RegionProviderChain::default_provider().or_else("us-east-1");
    let aws_config = aws_config::defaults(BehaviorVersion::latest())
        .region(region_provider)
        .load()
        .await;

    let dynamodb = aws_sdk_dynamodb::Client::new(&aws_config);

    // Create a shared state to pass to the handler
    let state = AppState {
        dynamodb: Arc::new(dynamodb),
        config,
    };

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

    // Set up a CORS layer
    let cors_layer = CorsLayer::new()
        .allow_headers([
            ACCEPT,
            ACCEPT_ENCODING,
            AUTHORIZATION,
            CONTENT_TYPE,
            ORIGIN,
        ])
        .allow_methods(tower_http::cors::Any)
        .allow_origin(tower_http::cors::Any);

    let compression_layer = CompressionLayer::new().gzip(true).deflate(true);

    // Create Axum app
    let app = Router::new()
        .route(
            "/:stage/records/:resource",
            get(list_records_handler).post(create_record_handler),
        )
        .route(
            "/:stage/records/:resource/:record_id",
            get(get_record_handler)
                .put(update_record_handler)
                .delete(delete_record_handler),
        )
        .route(
            "/:stage/records/:resource/many",
            get(get_many_records_handler),
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
        .layer(cors_layer)
        .layer(trace_layer)
        .layer(compression_layer)
        .with_state(state);

    // Provide the app to the lambda runtime
    let app = tower::ServiceBuilder::new()
        .layer(axum_aws_lambda::LambdaLayer::default())
        .service(app);

    lambda_http::run(app).await.unwrap();
}

fn get_table_name<'a>(state: &'a AppState, resource: &'a str) -> &'a str {
    match resource {
        "streams" => &state.config.streams_table,
        "episodes" => &state.config.episodes_table,
        "series" => &state.config.series_table,
        "video_clips" => &state.config.video_metadata_table,
        "profiles" => &state.config.profiles_table,
        _ => panic!("unsupported resource: {resource}"),
    }
}

// TODO make every function in main.rs change the key in the response to "id" if the resource is "video_clips" (use get_key_name)
fn get_key_name(resource: &str) -> &str {
    match resource {
        "video_clips" => "key",
        _ => "id",
    }
}

/// Lists records from the specified ``DynamoDB`` table based on the provided
/// query parameters.
///
/// # Arguments
///
/// * `state` - A reference to the shared resources containing the
///   ``DynamoDB`` client and configuration.
/// * `table_name` - The name of the ``DynamoDB`` table to scan.
/// * `query` - A hashmap containing the query parameters, including filters as
///   a JSON string.
///
/// # Returns
///
/// A `Result` containing a `Response` with the scanned items and the total
/// count, or an `Error`.
#[allow(clippy::option_if_let_else)]
async fn list_records_handler(
    Path(RequestPath { stage: _, resource }): Path<RequestPath>,
    Query(query): Query<HashMap<String, String>>,
    State(state): State<AppState>,
) -> impl IntoResponse {
    let table_name = get_table_name(&state, &resource);
    let key_name = get_key_name(&resource);

    tracing::info!("listing records from table: {table_name}");

    // Parse the query parameters
    let filters = match query.get("filter") {
        Some(filter) => match filter.as_str() {
            "" => serde_json::Map::new(),
            _ => match serde_json::from_str(filter) {
                Ok(filters) => filters,
                Err(e) => {
                    tracing::warn!("failed to parse filters: {e}");
                    return (
                        StatusCode::BAD_REQUEST,
                        [(header::CONTENT_TYPE, "application/json")],
                        Json(json!({
                            "message": "failed to parse filters",
                        })),
                    );
                }
            },
        },
        None => serde_json::Map::new(),
    };

    // Call the `list` function from the `dynamodb` module

    let cursor = match query.get("cursor") {
        Some(cursor) => match cursor.as_str() {
            "null" | "" => None,
            _ => Some(cursor.clone()),
        },
        None => None,
    };

    match dynamodb::list(
        &state.dynamodb,
        table_name,
        key_name,
        filters,
        dynamodb::PageOptions {
            cursor,
            limit: query
                .get("perPage")
                .and_then(|s| s.parse().ok())
                .unwrap_or(10),
        },
    )
    .await
    {
        Ok(list_result) => {
            // Build the response
            tracing::info!("successfully listed records");

            (
                StatusCode::OK,
                [(header::CONTENT_TYPE, "application/json")],
                Json(json!({
                    "items": list_result.items,
                    "cursor": list_result.cursor,
                })),
            )
        }
        Err(e) => {
            tracing::error!("failed to list records: {e}");

            (
                StatusCode::INTERNAL_SERVER_ERROR,
                [(header::CONTENT_TYPE, "application/json")],
                Json(json!({
                    "message": "failed to list records",
                })),
            )
        }
    }
}

async fn get_record_handler(
    Path(RequestPathWithId {
        stage: _,
        resource,
        record_id,
    }): Path<RequestPathWithId>,
    State(state): State<AppState>,
) -> impl IntoResponse {
    let table_name = get_table_name(&state, &resource);
    let key_name = get_key_name(&resource);

    match dynamodb::get(
        &state.dynamodb,
        table_name,
        key_name,
        record_id.as_str(),
    )
    .await
    {
        Ok(result) => result.0.map_or_else(
            || {
                (
                    StatusCode::NOT_FOUND,
                    [(header::CONTENT_TYPE, "application/json")],
                    Json(json!({
                        "message": "record not found",
                    })),
                )
            },
            |record| {
                (
                    StatusCode::OK,
                    [(header::CONTENT_TYPE, "application/json")],
                    Json(record),
                )
            },
        ),
        Err(e) => {
            tracing::error!("failed to get record: {e}");

            (
                StatusCode::INTERNAL_SERVER_ERROR,
                [(header::CONTENT_TYPE, "application/json")],
                Json(json!({
                    "message": "failed to get record",
                })),
            )
        }
    }
}

async fn create_record_handler(
    Path(RequestPath { stage: _, resource }): Path<RequestPath>,
    State(state): State<AppState>,
    Json(payload): Json<serde_json::Value>,
) -> impl IntoResponse {
    let table_name = get_table_name(&state, &resource);

    match dynamodb::create(&state.dynamodb, table_name, &payload).await {
        Ok(()) => (
            StatusCode::CREATED,
            [(header::CONTENT_TYPE, "application/json")],
            Json(payload),
        ),
        Err(e) => {
            tracing::error!("failed to create record: {e}");

            (
                StatusCode::INTERNAL_SERVER_ERROR,
                [(header::CONTENT_TYPE, "application/json")],
                Json(json!({
                    "message": "failed to create record",
                })),
            )
        }
    }
}

async fn update_record_handler(
    Path(RequestPathWithId {
        stage: _,
        resource,
        record_id,
    }): Path<RequestPathWithId>,
    State(state): State<AppState>,
    Json(payload): Json<serde_json::Value>,
) -> impl IntoResponse {
    let table_name = get_table_name(&state, &resource);
    let key_name = get_key_name(&resource);

    match dynamodb::update(
        &state.dynamodb,
        table_name,
        key_name,
        record_id.as_str(),
        &payload,
    )
    .await
    {
        Ok(response) => (
            StatusCode::OK,
            [(header::CONTENT_TYPE, "application/json")],
            Json(response),
        ),
        Err(e) => {
            tracing::error!("failed to update record: {e}");

            (
                StatusCode::INTERNAL_SERVER_ERROR,
                [(header::CONTENT_TYPE, "application/json")],
                Json(json!({
                    "message": "failed to update record",
                })),
            )
        }
    }
}

async fn delete_record_handler(
    Path(RequestPathWithId {
        stage: _,
        resource,
        record_id,
    }): Path<RequestPathWithId>,
    State(state): State<AppState>,
) -> impl IntoResponse {
    let table_name = get_table_name(&state, &resource);
    let key_name = get_key_name(&resource);

    match dynamodb::delete(
        &state.dynamodb,
        table_name,
        key_name,
        record_id.as_str(),
    )
    .await
    {
        Ok(()) => (
            StatusCode::NO_CONTENT,
            [(header::CONTENT_TYPE, "application/json")],
            Json(json!({})),
        ),
        Err(e) => {
            tracing::error!("failed to delete record: {e}");

            (
                StatusCode::INTERNAL_SERVER_ERROR,
                [(header::CONTENT_TYPE, "application/json")],
                Json(json!({
                    "message": "failed to delete record",
                })),
            )
        }
    }
}

async fn get_many_records_handler(
    Path(RequestPath { stage: _, resource }): Path<RequestPath>,
    State(state): State<AppState>,
    Query(query_params): Query<ManyQuery>,
) -> impl IntoResponse {
    let table_name = get_table_name(&state, &resource);
    let key_name = get_key_name(&resource);

    let ids = query_params
        .id
        .iter()
        .map(String::as_str)
        .collect::<Vec<_>>();
    match dynamodb::get_many(&state.dynamodb, table_name, key_name, &ids).await
    {
        Ok(items) => (
            StatusCode::OK,
            [(header::CONTENT_TYPE, "application/json")],
            Json(json!({ "items": items })),
        ),
        Err(e) => {
            tracing::error!("failed to batch get records: {e}");

            (
                StatusCode::INTERNAL_SERVER_ERROR,
                [(header::CONTENT_TYPE, "application/json")],
                Json(json!({ "message": "failed to batch get records" })),
            )
        }
    }
}
