/**
 * This is the main entrypoint for the `crud_api` lambda function.
 *
 * The function is responsible for handling the requests and responses for the
 * CRUD operations from the API Gateway.
 *
 */
use aws_sdk_dynamodb::Client;
use axum::{
    Json, Router,
    body::Body,
    extract::{Path, Query, State},
    http::{
        Request, StatusCode,
        header::{
            self, ACCEPT, ACCEPT_ENCODING, AUTHORIZATION, CONTENT_TYPE, ORIGIN,
        },
    },
    response::IntoResponse,
    routing::get,
};
use dynamodb::DynamoDbTableConfig;
use gt_axum::cognito::OptionalCognitoUserId;
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
    tasks_table: String,
    projects_table: String,
    chat_messages_table: String,
    stream_widgets_table: String,
}

#[derive(Debug, Clone)]
struct AppContext {
    dynamodb: Arc<Client>,
    config: Config,
}

impl gt_app::ContextProvider<Config> for AppContext {
    async fn new(config: Config, aws_config: aws_config::SdkConfig) -> Self {
        Self {
            config,
            dynamodb: Arc::new(aws_sdk_dynamodb::Client::new(&aws_config)),
        }
    }
}

#[derive(Debug, Deserialize)]
struct RequestPath {
    resource: String,
}

#[derive(Debug, Deserialize)]
struct RequestPathWithId {
    resource: String,
    record_id: String,
}

#[derive(Debug, Deserialize)]
struct RequestPathWithRelatedField {
    resource: String,
    related_field: String,
    id: String,
}

#[derive(Deserialize)]
struct ManyQuery {
    id: Vec<String>,
}

#[tokio::main]
async fn main() {
    // Initialize the application context
    let app_context = gt_app::create_app_context().await.unwrap();

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
            "/records/{resource}",
            get(list_records_handler).post(create_record_handler),
        )
        .route(
            "/records/{resource}/{record_id}",
            get(get_record_handler)
                .put(update_record_handler)
                .delete(delete_record_handler),
        )
        .route(
            "/records/{resource}/{related_field}/{id}",
            get(get_many_related_records_handler),
        )
        .route("/records/{resource}/many", get(get_many_records_handler))
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
        .with_state(app_context);

    // Provide the app to the lambda runtime
    let app = tower::ServiceBuilder::new()
        .layer(axum_aws_lambda::LambdaLayer::default().trim_stage())
        .service(app);

    lambda_http::run(app).await.unwrap();
}

fn get_table_config<'a>(
    state: &'a AppContext,
    resource: &'a str,
) -> DynamoDbTableConfig<'a> {
    match resource {
        "streams" => DynamoDbTableConfig {
            table: &state.config.streams_table,
            partition_key: "id",
            q_key: "title",
            indexes: vec![],
            user_scoped: false,
        },
        "episodes" => DynamoDbTableConfig {
            table: &state.config.episodes_table,
            partition_key: "id",
            q_key: "title",
            indexes: vec![],
            user_scoped: false,
        },
        "series" => DynamoDbTableConfig {
            table: &state.config.series_table,
            partition_key: "id",
            q_key: "title",
            indexes: vec![],
            user_scoped: false,
        },
        "video_clips" => DynamoDbTableConfig {
            table: &state.config.video_metadata_table,
            partition_key: "key",
            q_key: "key",
            indexes: vec!["stream_id"],
            user_scoped: false,
        },
        "profiles" => DynamoDbTableConfig {
            table: &state.config.profiles_table,
            partition_key: "id",
            q_key: "id",
            indexes: vec![],
            user_scoped: false,
        },
        "tasks" => DynamoDbTableConfig {
            table: &state.config.tasks_table,
            partition_key: "id",
            q_key: "title",
            indexes: vec![],
            user_scoped: false,
        },
        "projects" => DynamoDbTableConfig {
            table: &state.config.projects_table,
            partition_key: "id",
            q_key: "title",
            indexes: vec![],
            user_scoped: false,
        },
        "chat_messages" => DynamoDbTableConfig {
            table: &state.config.chat_messages_table,
            partition_key: "user_id",
            q_key: "timestamp",
            indexes: vec!["channel_id"],
            user_scoped: false,
        },
        "stream_widgets" => DynamoDbTableConfig {
            table: &state.config.stream_widgets_table,
            partition_key: "id",
            q_key: "title",
            indexes: vec!["type", "active"],
            user_scoped: true,
        },
        _ => panic!("unsupported resource: {resource}"),
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
    OptionalCognitoUserId(user_id): OptionalCognitoUserId,
    Path(RequestPath { resource }): Path<RequestPath>,
    Query(query): Query<HashMap<String, String>>,
    State(state): State<AppContext>,
) -> impl IntoResponse {
    let table_config = get_table_config(&state, &resource);

    tracing::info!("listing records from table: {0}", table_config.table);

    // Parse the query parameters
    let mut filters = match query.get("filter") {
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

    // If user_scoped, add user_id filter
    if table_config.user_scoped {
        if let Some(user) = user_id {
            filters.insert("user_id".to_string(), json!(user));
        } else {
            return (
                StatusCode::UNAUTHORIZED,
                [(header::CONTENT_TYPE, "application/json")],
                Json(json!({
                    "message": "Unauthorized",
                })),
            );
        }
    }

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
        &table_config,
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
    OptionalCognitoUserId(user_id): OptionalCognitoUserId,
    Path(RequestPathWithId {
        resource,
        record_id,
    }): Path<RequestPathWithId>,
    State(state): State<AppContext>,
) -> impl IntoResponse {
    let table_config = get_table_config(&state, &resource);

    match dynamodb::get(&state.dynamodb, &table_config, record_id.as_str())
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
                // If user_scoped, verify ownership
                if table_config.user_scoped {
                    if let Some(user) = user_id {
                        if record.get("user_id").and_then(|v| v.as_str())
                            != Some(&user)
                        {
                            return (
                                StatusCode::FORBIDDEN,
                                [(header::CONTENT_TYPE, "application/json")],
                                Json(json!({
                                    "message": "Forbidden",
                                })),
                            );
                        }
                    } else {
                        return (
                            StatusCode::UNAUTHORIZED,
                            [(header::CONTENT_TYPE, "application/json")],
                            Json(json!({
                                "message": "Unauthorized",
                            })),
                        );
                    }
                }
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
    OptionalCognitoUserId(user_id): OptionalCognitoUserId,
    Path(RequestPath { resource }): Path<RequestPath>,
    State(state): State<AppContext>,
    Json(payload): Json<serde_json::Value>,
) -> impl IntoResponse {
    let table_config = get_table_config(&state, &resource);

    let mut payload = match payload {
        serde_json::Value::Object(map) => {
            vec![serde_json::Value::Object(map)]
        }
        serde_json::Value::Array(array) => array,
        _ => {
            return (
                StatusCode::BAD_REQUEST,
                [(header::CONTENT_TYPE, "application/json")],
                Json(json!({
                    "message": "invalid payload: expected object or array",
                })),
            );
        }
    };

    // If user_scoped, inject user_id
    if table_config.user_scoped {
        if let Some(user) = user_id {
            for item in &mut payload {
                if let serde_json::Value::Object(map) = item {
                    map.insert("user_id".to_string(), json!(user));
                }
            }
        } else {
            return (
                StatusCode::UNAUTHORIZED,
                [(header::CONTENT_TYPE, "application/json")],
                Json(json!({
                    "message": "Unauthorized",
                })),
            );
        }
    }

    match dynamodb::create(
        &state.dynamodb,
        &table_config,
        payload.iter().collect(),
    )
    .await
    {
        Ok(items) => {
            if payload.len() == 1 {
                (
                    StatusCode::CREATED,
                    [(header::CONTENT_TYPE, "application/json")],
                    Json(items[0].clone()),
                )
            } else {
                (
                    StatusCode::CREATED,
                    [(header::CONTENT_TYPE, "application/json")],
                    Json(json!({ "items": items })),
                )
            }
        }
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
    OptionalCognitoUserId(user_id): OptionalCognitoUserId,
    Path(RequestPathWithId {
        resource,
        record_id,
    }): Path<RequestPathWithId>,
    State(state): State<AppContext>,
    Json(payload): Json<serde_json::Value>,
) -> impl IntoResponse {
    let table_config = get_table_config(&state, &resource);

    // If user_scoped, verify ownership before updating
    if table_config.user_scoped {
        match dynamodb::get(&state.dynamodb, &table_config, record_id.as_str())
            .await
        {
            Ok(result) => {
                if let Some(record) = result.0 {
                    if let Some(user) = &user_id {
                        if record.get("user_id").and_then(|v| v.as_str())
                            != Some(user)
                        {
                            return (
                                StatusCode::FORBIDDEN,
                                [(header::CONTENT_TYPE, "application/json")],
                                Json(json!({
                                    "message": "Forbidden",
                                })),
                            );
                        }
                    } else {
                        return (
                            StatusCode::UNAUTHORIZED,
                            [(header::CONTENT_TYPE, "application/json")],
                            Json(json!({
                                "message": "Unauthorized",
                            })),
                        );
                    }
                } else {
                    return (
                        StatusCode::NOT_FOUND,
                        [(header::CONTENT_TYPE, "application/json")],
                        Json(json!({
                            "message": "record not found",
                        })),
                    );
                }
            }
            Err(e) => {
                tracing::error!("failed to verify record ownership: {e}");
                return (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    [(header::CONTENT_TYPE, "application/json")],
                    Json(json!({
                        "message": "failed to verify record ownership",
                    })),
                );
            }
        }
    }

    // For user_scoped resources, remove user_id from payload to prevent privilege escalation
    let mut sanitized_payload = payload.clone();
    if table_config.user_scoped {
        if let Some(obj) = sanitized_payload.as_object_mut() {
            obj.remove("user_id");
        }
    }

    match dynamodb::update(
        &state.dynamodb,
        &table_config,
        &record_id,
        &sanitized_payload,
    )
    .await
    {
        Ok(response) => (
            StatusCode::OK,
            [(header::CONTENT_TYPE, "application/json")],
            Json(response),
        ),
        Err(e) => {
            tracing::error!("failed to update record: {:?}", e);

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
    OptionalCognitoUserId(user_id): OptionalCognitoUserId,
    Path(RequestPathWithId {
        resource,
        record_id,
    }): Path<RequestPathWithId>,
    State(state): State<AppContext>,
) -> impl IntoResponse {
    let table_config = get_table_config(&state, &resource);

    // If user_scoped, verify ownership before deleting
    if table_config.user_scoped {
        match dynamodb::get(&state.dynamodb, &table_config, record_id.as_str())
            .await
        {
            Ok(result) => {
                if let Some(record) = result.0 {
                    if let Some(user) = &user_id {
                        if record.get("user_id").and_then(|v| v.as_str())
                            != Some(user)
                        {
                            return (
                                StatusCode::FORBIDDEN,
                                [(header::CONTENT_TYPE, "application/json")],
                                Json(json!({
                                    "message": "Forbidden",
                                })),
                            );
                        }
                    } else {
                        return (
                            StatusCode::UNAUTHORIZED,
                            [(header::CONTENT_TYPE, "application/json")],
                            Json(json!({
                                "message": "Unauthorized",
                            })),
                        );
                    }
                } else {
                    return (
                        StatusCode::NOT_FOUND,
                        [(header::CONTENT_TYPE, "application/json")],
                        Json(json!({
                            "message": "record not found",
                        })),
                    );
                }
            }
            Err(e) => {
                tracing::error!("failed to verify record ownership: {e}");
                return (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    [(header::CONTENT_TYPE, "application/json")],
                    Json(json!({
                        "message": "failed to verify record ownership",
                    })),
                );
            }
        }
    }

    match dynamodb::delete(&state.dynamodb, &table_config, record_id.as_str())
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
    OptionalCognitoUserId(user_id): OptionalCognitoUserId,
    Path(RequestPath { resource }): Path<RequestPath>,
    State(state): State<AppContext>,
    Query(query_params): Query<ManyQuery>,
) -> impl IntoResponse {
    let table_config = get_table_config(&state, &resource);

    let ids = query_params
        .id
        .iter()
        .map(String::as_str)
        .collect::<Vec<_>>();
    match dynamodb::get_many(&state.dynamodb, &table_config, &ids).await {
        Ok(mut items) => {
            // If user_scoped, filter items by user_id
            if table_config.user_scoped {
                if let Some(user) = user_id {
                    items.retain(|item| {
                        item.get("user_id").and_then(|v| v.as_str())
                            == Some(&user)
                    });
                } else {
                    return (
                        StatusCode::UNAUTHORIZED,
                        [(header::CONTENT_TYPE, "application/json")],
                        Json(json!({
                            "message": "Unauthorized",
                        })),
                    );
                }
            }
            (
                StatusCode::OK,
                [(header::CONTENT_TYPE, "application/json")],
                Json(json!({ "items": items })),
            )
        }
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

async fn get_many_related_records_handler(
    OptionalCognitoUserId(user_id): OptionalCognitoUserId,
    Path(RequestPathWithRelatedField {
        resource,
        related_field,
        id,
    }): Path<RequestPathWithRelatedField>,
    State(state): State<AppContext>,
) -> impl IntoResponse {
    let table_config = get_table_config(&state, &resource);

    // If user_scoped, reject for now (not commonly used)
    if table_config.user_scoped {
        if user_id.is_none() {
            return (
                StatusCode::UNAUTHORIZED,
                [(header::CONTENT_TYPE, "application/json")],
                Json(json!({
                    "message": "Unauthorized",
                })),
            );
        }
    }

    // validate the related field against the table configuration
    if !table_config.indexes.contains(&related_field.as_str()) {
        return (
            StatusCode::BAD_REQUEST,
            [(header::CONTENT_TYPE, "application/json")],
            Json(json!({
                "message": "invalid related field",
            })),
        );
    }

    match dynamodb::query(
        &state.dynamodb,
        &table_config,
        related_field.as_str(),
        json!(id),
    )
    .await
    {
        Ok(result) => (
            StatusCode::OK,
            [(header::CONTENT_TYPE, "application/json")],
            Json(json!({
                "items": result.items,
                "cursor": result.cursor,
            })),
        ),
        Err(e) => {
            tracing::error!("failed to query related records: {e}");

            (
                StatusCode::INTERNAL_SERVER_ERROR,
                [(header::CONTENT_TYPE, "application/json")],
                Json(json!({
                    "message": "failed to query related records",
                })),
            )
        }
    }
}
