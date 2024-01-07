use axum::extract::{Path, State};
use axum::Json;
use axum::{http::StatusCode, response::IntoResponse, routing::get};
use common_api_lib;
use dotenvy;
use redis::{Commands, ConnectionLike};
use serde::{Deserialize, Serialize};
use serde_json::json;
use tracing::instrument;

#[derive(Clone, Debug)]
struct AppState {
    redis: redis::Client,
}

#[tokio::main]
async fn main() -> Result<(), axum::BoxError> {
    let state = AppState {
        redis: redis::Client::open(dotenvy::var("REDIS_URL").expect("REDIS_URL must be set"))?,
    };

    common_api_lib::run(state, |app| {
        app.route("/tasks", get(get_list_handler).post(create_handler))
            .route(
                "/tasks/:record_id",
                get(get_one_handler)
                    .put(update_handler)
                    .delete(delete_handler),
            )
    })
    .await
}

#[instrument]
async fn get_list_handler(State(state): State<AppState>) -> impl IntoResponse {
    let mut con = match state.redis.get_connection() {
        Ok(con) => con,
        Err(e) => {
            tracing::error!("Failed to get redis connection: {}", e);
            return (StatusCode::INTERNAL_SERVER_ERROR, axum::Json(json!({}))).into_response();
        }
    };

    // get the list of records from redis using the key pattern with scan_match
    let keys: Vec<String> = match con.scan_match("task:[0-9]*") {
        Ok(keys) => keys.collect(),
        Err(e) => {
            tracing::error!("Failed to get task keys: {}", e);
            return (StatusCode::INTERNAL_SERVER_ERROR, axum::Json(json!({}))).into_response();
        }
    };

    // TODO get the records from redis

    // return the list of records
    (StatusCode::OK, axum::Json(json!(keys))).into_response()
}

#[derive(Deserialize, Debug)]
struct CreateTaskInput {
    url: String,
    payload: serde_json::Value,
    data_key: String,
}

#[instrument]
async fn create_handler(
    State(state): State<AppState>,
    Json(body): Json<CreateTaskInput>,
) -> impl IntoResponse {
    // TODO move this to an axum extractor???
    let mut con = match state.redis.get_connection() {
        Ok(con) => con,
        Err(e) => {
            tracing::error!("Failed to get redis connection: {}", e);
            return (StatusCode::INTERNAL_SERVER_ERROR, axum::Json(json!({}))).into_response();
        }
    };

    let queue_name = match dotenvy::var("QUEUE_NAME") {
        Ok(queue_name) => queue_name,
        Err(e) => {
            tracing::error!("Failed to get QUEUE_NAME: {}", e);
            return (StatusCode::INTERNAL_SERVER_ERROR, axum::Json(json!({}))).into_response();
        }
    };

    // generate a unique id for the task by incrementing the task counter
    let id: u64 = match con.incr("task:counter", 1) {
        Ok(id) => {
            tracing::info!("Generated task id: {}", id);
            id
        }
        Err(e) => {
            tracing::error!("Failed to generate task id: {}", e);
            return (StatusCode::INTERNAL_SERVER_ERROR, axum::Json(json!({}))).into_response();
        }
    };

    // key pattern = task:(next-id)
    let key = generate_task_key(id);

    // create task record as a hash in redis with a unique id
    match con.req_command(
        redis::cmd("HMSET")
            .arg(&key)
            .arg("id")
            .arg(id)
            .arg("status")
            .arg("queued")
            .arg("url")
            .arg(&body.url)
            .arg("payload")
            .arg(body.payload.to_string())
            .arg("data_key")
            .arg(&body.data_key),
    ) {
        Ok(_) => {
            tracing::info!("Created task record: {}", key);
        }
        Err(e) => {
            tracing::error!("Failed to create task record: {}", e);
            return (StatusCode::INTERNAL_SERVER_ERROR, axum::Json(json!({}))).into_response();
        }
    };

    // add the task key to the queue
    match con.lpush::<&std::string::String, &std::string::String, ()>(&queue_name, &key) {
        Ok(_) => {
            tracing::info!("Added task to queue: {}", queue_name);
        }
        Err(e) => {
            tracing::error!("Failed to add task to queue: {}", e);
            return (StatusCode::INTERNAL_SERVER_ERROR, axum::Json(json!({}))).into_response();
        }
    };

    // return the created task record
    (
        StatusCode::OK,
        axum::Json(json!({
            "id": id.to_string(),
            "url": body.url,
            "payload": body.payload,
        })),
    )
        .into_response()
}

#[derive(Serialize, Debug)]
enum TaskStatus {
    Queued,
    Processing,
    Complete,
    Failed,
}

#[derive(Serialize, Debug)]
struct Task {
    id: String,
    status: TaskStatus,
    data: Vec<serde_json::Value>,
}

#[instrument]
async fn get_one_handler(
    Path(record_id): Path<u64>,
    State(state): State<AppState>,
) -> impl IntoResponse {
    let mut con = match state.redis.get_connection() {
        Ok(con) => con,
        Err(e) => {
            tracing::error!("Failed to get redis connection: {}", e);
            return (StatusCode::INTERNAL_SERVER_ERROR, axum::Json(json!({}))).into_response();
        }
    };

    // get the record from redis
    let key = generate_task_key(record_id);

    let status: String = match con.hget(&key, "status") {
        Ok(status) => status,
        Err(e) => {
            tracing::error!("Failed to get task record: {}", e);
            return (StatusCode::INTERNAL_SERVER_ERROR, axum::Json(json!({}))).into_response();
        }
    };

    let status = match status.as_str() {
        "queued" => TaskStatus::Queued,
        "processing" => TaskStatus::Processing,
        "complete" => TaskStatus::Complete,
        "failed" => TaskStatus::Failed,
        _ => {
            tracing::error!("Invalid task status: {}", status);
            return (StatusCode::INTERNAL_SERVER_ERROR, axum::Json(json!({}))).into_response();
        }
    };

    // get the data list from redis if it exists
    let data_key = generate_task_data_key(record_id);

    let data: Vec<String> = match con.lrange(&data_key, 0, -1) {
        Ok(data) => data,
        Err(e) => {
            tracing::error!("Failed to get data list: {}", e);
            return (StatusCode::INTERNAL_SERVER_ERROR, axum::Json(json!({}))).into_response();
        }
    };

    // parse the JSON list in each item in the data list
    let data: Vec<serde_json::Value> = data
        .iter()
        .map(|item| serde_json::from_str::<Vec<serde_json::Value>>(item).unwrap_or(vec![]))
        .flatten()
        .collect();

    // return the record
    let record = Task {
        id: record_id.to_string(),

        status,

        data,
    };

    (StatusCode::OK, axum::Json(json!(record))).into_response()
}

#[instrument]
async fn update_handler() -> impl IntoResponse {
    // TODO get the record id from the request path
    // TODO update the record in redis
    // TODO return the updated record
    (StatusCode::OK, axum::Json(json!({}))).into_response()
}

#[instrument]
async fn delete_handler() -> impl IntoResponse {
    // TODO get the record id from the request path
    // TODO delete the record from redis
    // TODO return the right status code
    (StatusCode::OK, axum::Json(json!({}))).into_response()
}

fn generate_task_key(id: u64) -> String {
    format!("task:{}", id)
}

fn generate_task_data_key(id: u64) -> String {
    format!("task:{}:data", id)
}
