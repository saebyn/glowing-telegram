use axum::extract::State;
use axum::Json;
use axum::{http::StatusCode, response::IntoResponse, routing::get};
use common_api_lib;
use dotenvy;
use redis::{Commands, ConnectionLike};
use serde::Deserialize;
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
    let id: i64 = match con.incr("task:counter", 1) {
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
    let key = format!("task:{}", id);

    // create task record as a hash in redis with a unique id
    match con.req_command(
        redis::cmd("HMSET")
            .arg(&key)
            .arg("id")
            .arg(id)
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
            "id": id,
            "url": body.url,
            "payload": body.payload,

        })),
    )
        .into_response()
}

#[instrument]
async fn get_one_handler() -> impl IntoResponse {
    // TODO get the record id from the request path
    // TODO get the record from redis
    // TODO return the record
    (StatusCode::OK, axum::Json(json!({}))).into_response()
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
