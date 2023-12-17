use axum::extract::State;
use axum::{http::StatusCode, response::IntoResponse, routing::get};
use common_api_lib;
use dotenvy;
use redis::{Commands, ConnectionLike};
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
async fn get_list_handler() -> impl IntoResponse {
    (StatusCode::OK, axum::Json(json!({}))).into_response()
}

#[instrument]
async fn create_handler(State(state): State<AppState>) -> impl IntoResponse {
    // TODO get request body contents and use that

    // TODO move this to an axum extractor???
    let mut con = match state.redis.get_connection() {
        Ok(con) => con,
        Err(e) => {
            tracing::error!("Failed to get redis connection: {}", e);
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
    // TODO use the request body contents here
    match con.req_command(
        redis::cmd("HMSET")
            .arg(&key)
            .arg("id")
            .arg(id)
            .arg("name")
            .arg("test"),
    ) {
        Ok(_) => {
            tracing::info!("Created task record: {}", key);
        }
        Err(e) => {
            tracing::error!("Failed to create task record: {}", e);
            return (StatusCode::INTERNAL_SERVER_ERROR, axum::Json(json!({}))).into_response();
        }
    };
    // publish a message to the task channel
    match con.publish::<&str, std::string::String, ()>("task", key.clone()) {
        Ok(_) => {
            tracing::info!("Published task record: {}", key);
        }
        Err(e) => {
            tracing::error!("Failed to publish task record: {}", e);
            return (StatusCode::INTERNAL_SERVER_ERROR, axum::Json(json!({}))).into_response();
        }
    };

    // TODO return the created task record
    (StatusCode::OK, axum::Json(json!({}))).into_response()
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
