use std::collections::HashMap;

use redis::{Commands, ConnectionLike};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct Task {
    pub key: String,
    pub id: u64,
    pub url: String,
    pub payload: serde_json::Value,
    pub data_key: String,
    pub title: String,
    pub status: TaskStatus,
    pub last_updated: String,
}

impl From<HashMap<String, String>> for Task {
    fn from(data: HashMap<String, String>) -> Self {
        let id = data["id"].parse().expect("Failed to parse task id");

        Task {
            key: generate_task_key(id),
            id,
            title: data.get("title").unwrap_or(&"".to_string()).clone(),
            url: data["url"].clone(),
            payload: serde_json::from_str(&data["payload"]).expect("Failed to parse payload"),
            data_key: data["data_key"].clone(),
            status: TaskStatus::from(data["status"].clone()),
            last_updated: data.get("last_updated").unwrap_or(&"".to_string()).clone(),
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub enum TaskStatus {
    Queued,
    Processing,
    Complete,
    Failed,
    Invalid,
}

impl TaskStatus {
    pub fn as_str(&self) -> &str {
        match self {
            TaskStatus::Queued => "queued",
            TaskStatus::Processing => "processing",
            TaskStatus::Complete => "complete",
            TaskStatus::Failed => "failed",
            TaskStatus::Invalid => "invalid",
        }
    }
}

impl From<String> for TaskStatus {
    fn from(s: String) -> Self {
        match s.as_str() {
            "queued" => TaskStatus::Queued,
            "processing" => TaskStatus::Processing,
            "complete" => TaskStatus::Complete,
            "failed" => TaskStatus::Failed,
            _ => TaskStatus::Invalid,
        }
    }
}

// TODO make this private
pub fn generate_task_key(id: u64) -> String {
    format!("task:item:{}", id)
}

// TODO make this private
pub fn generate_task_data_key(id: u64) -> String {
    format!("task:data:{}", id)
}

pub fn create_task(
    con: &mut redis::Connection,
    id: u64,
    title: &str,
    url: &str,
    payload: serde_json::Value,
    data_key: &str,
) -> Result<Task, redis::RedisError> {
    let key = generate_task_key(id);

    let now = chrono::Utc::now().to_rfc3339();

    match con.req_command(
        redis::cmd("HMSET")
            .arg(&key)
            .arg("id")
            .arg(id)
            .arg("title")
            .arg(title)
            .arg("status")
            .arg(TaskStatus::Queued.as_str())
            .arg("url")
            .arg(url)
            .arg("payload")
            .arg(&payload.to_string())
            .arg("data_key")
            .arg(data_key)
            .arg("last_updated")
            .arg(&now),
    ) {
        Ok(_) => Ok(Task {
            key,
            id,
            title: title.to_string(),
            url: url.to_string(),
            payload,
            data_key: data_key.to_string(),
            status: TaskStatus::Queued,
            last_updated: now,
        }),
        Err(e) => Err(e),
    }
}

// TODO return a Result
pub fn update_task_status(con: &mut redis::Connection, task: &Task, status: TaskStatus) {
    let _: () = con
        .hset(task.key.clone(), "status", status.as_str())
        .expect("Failed to update task status");

    let _: () = con
        .hset(
            task.key.clone(),
            "last_updated",
            chrono::Utc::now().to_rfc3339(),
        )
        .expect("Failed to update task last_updated");
}

// TODO return a Result
pub fn pop_task(con: &mut redis::Connection, queue_name: &str) -> Task {
    let temp_queue_name = format!("{}:temp", queue_name);

    let task_key: String = con
        .blmove(
            queue_name,
            &temp_queue_name,
            redis::Direction::Right,
            redis::Direction::Left,
            0.0,
        )
        .expect("Failed to get task from queue");

    let task_data: HashMap<String, String> = con
        .hgetall(&task_key)
        .expect("Failed to get task data from redis");

    Task {
        key: task_key,
        id: task_data["id"].parse().expect("Failed to parse task id"),
        title: task_data["title"].clone(),
        url: task_data["url"].clone(),
        payload: serde_json::from_str(&task_data["payload"]).expect("Failed to parse payload"),
        data_key: task_data["data_key"].clone(),
        status: TaskStatus::from(task_data["status"].clone()),
        last_updated: task_data["last_updated"].clone(),
    }
}

// TODO return a Result
pub fn remove_task_from_temp_queue(con: &mut redis::Connection, task: &Task) {
    let temp_queue_name = format!("{}:temp", task.key);

    let _: () = con
        .lrem(&temp_queue_name, 1, task.key.clone())
        .expect("Failed to remove task from temp queue");
}
