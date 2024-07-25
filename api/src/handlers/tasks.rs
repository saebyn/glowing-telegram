use std::collections::HashMap;

use axum::extract::{Path, State};

use axum::extract::ws::{Message, WebSocket, WebSocketUpgrade};
use axum::http::header;
use axum::Json;
use axum::{http::StatusCode, response::IntoResponse};
use redis::Commands;
use serde::Serialize;
use serde_json::json;
use tracing::instrument;

use task_worker::{
    create_task, get_task, get_task_data, queue_task, Task, TaskStatus,
};

use crate::state::AppState;
use crate::task::TaskRequest;

#[instrument]
pub async fn get_list_handler(
    State(state): State<AppState>,
) -> impl IntoResponse {
    let mut con = match state.redis.get_connection() {
        Ok(con) => con,
        Err(e) => {
            tracing::error!("Failed to get redis connection: {}", e);
            return (StatusCode::INTERNAL_SERVER_ERROR, axum::Json(json!({})))
                .into_response();
        }
    };

    // get the list of records from redis using the key pattern with scan_match
    let keys: Vec<String> = match con.scan_match("task:item:[0-9]*") {
        Ok(keys) => keys.collect(),
        Err(e) => {
            tracing::error!("Failed to get task keys: {}", e);
            return (StatusCode::INTERNAL_SERVER_ERROR, axum::Json(json!({})))
                .into_response();
        }
    };

    //  get the records from redis using the keys
    let records: Vec<serde_json::Value> = keys
        .iter()
        .map::<HashMap<String, String>, _>(|key| match con.hgetall(key) {
            Ok(record) => record,
            Err(e) => {
                tracing::error!("Failed to get task record: {}", e);
                HashMap::new()
            }
        })
        .flat_map(|record| record.try_into().ok())
        .map(|record: Task| {
            json!(
                {
                    "id": record.id,
                    "title": record.title,
                    "url": record.url,
                    "status": record.status.as_str(),
                    "last_updated": record.last_updated.to_rfc3339(),
                    "has_next_task": record.next_task.is_some(),
                }
            )
        })
        .collect();

    let pagination_info = format!(
        "{} {start}-{stop}/{total}",
        "tasks",
        start = 0,
        stop = records.len(),
        total = records.len()
    );

    // return the list of records
    (
        [
            (header::CONTENT_RANGE, pagination_info),
            (header::CONTENT_TYPE, "application/json".to_string()),
        ],
        axum::Json(json!(records)),
    )
        .into_response()
}

#[instrument]
pub async fn create_handler(
    State(state): State<AppState>,
    Json(body): Json<TaskRequest>,
) -> impl IntoResponse {
    let mut con = match state.redis.get_connection() {
        Ok(con) => con,
        Err(e) => {
            tracing::error!("Failed to get redis connection: {}", e);
            return (StatusCode::INTERNAL_SERVER_ERROR, axum::Json(json!({})))
                .into_response();
        }
    };

    let queue_name = match dotenvy::var("QUEUE_NAME") {
        Ok(queue_name) => queue_name,
        Err(e) => {
            tracing::error!("Failed to get QUEUE_NAME: {}", e);
            return (StatusCode::INTERNAL_SERVER_ERROR, axum::Json(json!({})))
                .into_response();
        }
    };

    // create task record as a hash in redis with a unique id
    let task = match create_task(
        &mut con,
        &body.title,
        &body.url,
        body.payload.clone(),
        &body.data_key,
        body.next_task.clone(),
        None,
    ) {
        Ok(task) => {
            tracing::info!("Created task record: {}", task.key);
            task
        }
        Err(e) => {
            tracing::error!("Failed to create task record: {}", e);
            return (StatusCode::INTERNAL_SERVER_ERROR, axum::Json(json!({})))
                .into_response();
        }
    };

    // add the task key to the queue
    match queue_task(&mut con, &queue_name, &task) {
        Ok(_) => {
            tracing::info!("Added task to queue: {}", queue_name);
        }
        Err(_) => {
            return (StatusCode::INTERNAL_SERVER_ERROR, axum::Json(json!({})))
                .into_response();
        }
    };

    // return the created task record
    (
        StatusCode::OK,
        axum::Json(json!({
            "id": task.id.to_string(),
            "url": body.url,
            "payload": body.payload,
        })),
    )
        .into_response()
}

#[derive(Serialize, Debug)]
struct TaskOutput {
    id: String,
    status: TaskStatus,
    last_updated: String,
    data: Vec<serde_json::Value>,
}

#[instrument]
pub async fn get_one_handler(
    Path(record_id): Path<u64>,
    State(state): State<AppState>,
) -> impl IntoResponse {
    let mut con = match state.redis.get_connection() {
        Ok(con) => con,
        Err(e) => {
            tracing::error!("Failed to get redis connection: {}", e);
            return (StatusCode::INTERNAL_SERVER_ERROR, axum::Json(json!({})))
                .into_response();
        }
    };

    // get the record from redis
    let task = match get_task(&mut con, record_id) {
        Ok(task) => task,
        Err(e) => {
            tracing::error!("Failed to get task record: {}", e);
            return (StatusCode::INTERNAL_SERVER_ERROR, axum::Json(json!({})))
                .into_response();
        }
    };

    let data = match get_task_data(&mut con, record_id) {
        Ok(data) => data,
        Err(e) => {
            tracing::error!("Failed to get task data: {}", e);
            return (StatusCode::INTERNAL_SERVER_ERROR, axum::Json(json!({})))
                .into_response();
        }
    };

    // return the record
    let record = TaskOutput {
        id: record_id.to_string(),

        status: task.status,
        last_updated: task.last_updated.to_rfc3339(),

        data,
    };

    (StatusCode::OK, axum::Json(json!(record))).into_response()
}

#[instrument]
pub async fn update_handler() -> impl IntoResponse {
    // TODO get the record id from the request path
    // TODO update the record in redis
    // TODO return the updated record
    (StatusCode::OK, axum::Json(json!({}))).into_response()
}

#[instrument]
pub async fn delete_handler() -> impl IntoResponse {
    // TODO get the record id from the request path
    // TODO delete the record from redis
    // TODO return the right status code
    (StatusCode::OK, axum::Json(json!({}))).into_response()
}

#[instrument]
pub async fn ws_handler(
    ws: WebSocketUpgrade,
    State(state): State<AppState>,
) -> impl IntoResponse {
    ws.on_upgrade(move |socket| handle_socket(socket, state.redis))
}

#[instrument]
pub async fn handle_socket(mut socket: WebSocket, redis: redis::Client) {
    // send a ping (unsupported by some browsers) just to kick things off and get a response
    if socket.send(Message::Ping(vec![])).await.is_ok() {
        println!("Pinged client...");
    } else {
        println!("Could not send ping client!");
        // no Error here since the only thing we can do is to close the connection.
        // If we can not send messages, there is no way to salvage the statemachine anyway.
        return;
    }

    let mut con = match redis.get_connection() {
        Ok(con) => con,
        Err(e) => {
            tracing::error!("Failed to get redis connection: {}", e);
            return;
        }
    };

    let mut pubsub = con.as_pubsub();

    // subscribe to the task channel
    pubsub.subscribe("task").unwrap();

    // listen for messages on the task channel
    loop {
        let msg = pubsub.get_message().unwrap();
        let payload: String = msg.get_payload().unwrap();
        println!("Received: {}", payload);

        // send the message to the client
        if socket.send(Message::Text(payload.clone())).await.is_ok() {
            println!("Sent: {}", payload);
        } else {
            println!("Could not send message!");
            return;
        }
    }
}
