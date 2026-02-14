use aws_sdk_dynamodb::Client as DynamoDbClient;
use aws_sdk_s3::Client as S3Client;
use axum::{
    Json, Router,
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    routing::get,
};
use gt_axum::cognito::CognitoUserId;
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, sync::Arc};
use thiserror::Error;
use tower_http::trace::TraceLayer;

mod config;
mod s3_status;

use config::StorageCostConfig;
use s3_status::{aggregate_s3_objects_info, calculate_costs};

#[derive(Debug, Deserialize, Clone)]
struct Config {
    streams_table: String,
    video_archive_bucket: String,
    storage_cost_config_json: String,
}

#[derive(Debug, Clone)]
struct AppContext {
    dynamodb: Arc<DynamoDbClient>,
    s3: Arc<S3Client>,
    config: Config,
    cost_config: StorageCostConfig,
}

impl gt_app::ContextProvider<Config> for AppContext {
    async fn new(config: Config, aws_config: aws_config::SdkConfig) -> Self {
        let cost_config =
            serde_json::from_str(&config.storage_cost_config_json)
                .expect("Failed to parse storage cost config");

        Self {
            config,
            dynamodb: Arc::new(DynamoDbClient::new(&aws_config)),
            s3: Arc::new(S3Client::new(&aws_config)),
            cost_config,
        }
    }
}

#[derive(Debug, Serialize)]
struct S3StatusResponse {
    exists: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    storage_class: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    size_bytes: Option<i64>,
    retrieval_required: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    estimated_retrieval_cost_usd: Option<HashMap<String, f64>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    estimated_retrieval_time_hours: Option<HashMap<String, f64>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    estimated_compute_cost_usd: Option<f64>,
}

#[derive(Debug, Error)]
enum ApiError {
    #[error("Stream not found")]
    StreamNotFound,
    #[error("S3 error: {0}")]
    S3Error(String),
    #[error("DynamoDB error: {0}")]
    DynamoDbError(String),
}

impl IntoResponse for ApiError {
    fn into_response(self) -> axum::response::Response {
        let (status, message) = match self {
            ApiError::StreamNotFound => {
                (StatusCode::NOT_FOUND, self.to_string())
            }
            ApiError::S3Error(_) | ApiError::DynamoDbError(_) => {
                (StatusCode::INTERNAL_SERVER_ERROR, self.to_string())
            }
        };

        (status, Json(serde_json::json!({ "error": message }))).into_response()
    }
}

async fn get_stream_s3_key(
    dynamodb: &DynamoDbClient,
    streams_table: &str,
    stream_id: &str,
) -> Result<Option<String>, ApiError> {
    let result = dynamodb
        .get_item()
        .table_name(streams_table)
        .key(
            "id",
            aws_sdk_dynamodb::types::AttributeValue::S(stream_id.to_string()),
        )
        .send()
        .await
        .map_err(|e| {
            tracing::error!(
                "DynamoDB GetItem error for stream {}: {}",
                stream_id,
                e
            );
            ApiError::DynamoDbError(
                "Failed to retrieve stream information".to_string(),
            )
        })?;

    if let Some(item) = result.item {
        // The prefix field contains the S3 key prefix for this stream
        if let Some(prefix_attr) = item.get("prefix") {
            if let Ok(prefix) = prefix_attr.as_s() {
                return Ok(Some(prefix.clone()));
            }
        }
    }

    Ok(None)
}

async fn handle_get_s3_status(
    State(ctx): State<Arc<AppContext>>,
    Path(stream_id): Path<String>,
    _user_id: CognitoUserId,
) -> Result<Json<S3StatusResponse>, ApiError> {
    // Get the stream from DynamoDB to verify it exists and get the S3 key
    let s3_key = get_stream_s3_key(
        &ctx.dynamodb,
        &ctx.config.streams_table,
        &stream_id,
    )
    .await?;

    let Some(s3_key) = s3_key else {
        return Err(ApiError::StreamNotFound);
    };

    // The prefix is a directory prefix, not a full object key
    // List ALL objects under the prefix to aggregate information
    // Use pagination to handle prefixes with >1000 objects
    let mut continuation_token: Option<String> = None;
    let mut all_objects = Vec::new();

    loop {
        let mut request = ctx
            .s3
            .list_objects_v2()
            .bucket(&ctx.config.video_archive_bucket)
            .prefix(&s3_key);

        if let Some(ref token) = continuation_token {
            request = request.continuation_token(token);
        }

        let list_output = request.send().await.map_err(|e| {
            tracing::error!(
                "S3 ListObjectsV2 error for prefix {}: {}",
                s3_key,
                e
            );
            ApiError::S3Error("Failed to list objects".to_string())
        })?;

        if let Some(mut contents) = list_output.contents {
            all_objects.append(&mut contents);
        }

        let is_truncated = list_output.is_truncated.unwrap_or(false);
        if !is_truncated {
            break;
        }

        continuation_token = list_output.next_continuation_token;
        if continuation_token.is_none() {
            // Defensive: if S3 marks the response as truncated but does not provide
            // a continuation token, break to avoid an infinite loop.
            break;
        }
    }

    // Aggregate information about all objects under the prefix
    let s3_info = aggregate_s3_objects_info(all_objects);

    // Calculate costs and times if objects exist
    let (retrieval_costs, retrieval_times, compute_cost) = if s3_info.exists {
        let costs = calculate_costs(&s3_info, &ctx.cost_config);
        (
            costs.retrieval_costs,
            costs.retrieval_times,
            Some(costs.compute_cost),
        )
    } else {
        (None, None, None)
    };

    let response = S3StatusResponse {
        exists: s3_info.exists,
        storage_class: s3_info.storage_class,
        size_bytes: s3_info.size_bytes,
        retrieval_required: s3_info.retrieval_required,
        estimated_retrieval_cost_usd: retrieval_costs,
        estimated_retrieval_time_hours: retrieval_times,
        estimated_compute_cost_usd: compute_cost,
    };

    Ok(Json(response))
}

#[tokio::main]
async fn main() {
    // Initialize the application context (which also initializes tracing)
    let app_context = gt_app::create_app_context::<AppContext, Config>()
        .await
        .expect("Failed to create app context");

    // Create the router
    let app = Router::new()
        .route(
            "/ingestion/streams/{id}/s3-status",
            get(handle_get_s3_status),
        )
        .layer(TraceLayer::new_for_http())
        .with_state(Arc::new(app_context));

    // Run the lambda
    gt_axum::run_lambda_app(app)
        .await
        .expect("Failed to run lambda");
}
