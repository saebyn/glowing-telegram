/**
 * Task functions
 *
 * This module is responsible for managing tasks in the task queue.
 *
 * The task queue is a list in Redis that contains the keys of tasks that need
 * to be processed.
 *
 * The task worker will pop a task from the queue and temporarily store it in a
 * temp queue (i.e. a processing list) while it is being processed.
 *
 * The task worker will then process the task by repeatedly calling the target
 * URL with the payload until the cursor returned by the target URL is null.
 *
 * If the target URL returns a 503 Service Unavailable status code, the task
 * will be marked as queued again, a retry timestamp will be added to the task
 * payload, and the task will be put back in the main queue.
 *
 * The task worker will also publish a message to the task channel whenever the
 * status of a task changes.
 */
use std::collections::HashMap;

use redis::{Commands, ConnectionLike};
use serde::{Deserialize, Serialize};
use serde_json::json;

#[derive(Debug, Serialize, Deserialize)]
pub struct Task {
    pub key: String,
    pub id: u64,
    pub url: String,
    pub payload: serde_json::Value,
    pub data_key: String,
    pub title: String,
    pub status: TaskStatus,

    #[serde(with = "chrono::serde::ts_seconds")]
    pub last_updated: chrono::DateTime<chrono::Utc>,

    #[serde(with = "chrono::serde::ts_seconds")]
    pub run_after: chrono::DateTime<chrono::Utc>,

    pub next_task: Option<TaskTemplate>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TaskTemplate {
    pub url: String,
    pub payload: serde_json::Value,
    pub data_key: String,
    pub title: String,
    pub next_task: Option<Box<TaskTemplate>>,
}

impl TryFrom<HashMap<String, String>> for TaskTemplate {
    type Error = &'static str;

    fn try_from(data: HashMap<String, String>) -> Result<Self, Self::Error> {
        let url = data.get("url").ok_or("Failed to get url")?.clone();
        let payload = match serde_json::from_str::<serde_json::Value>(
            data.get("payload").ok_or("Failed to get payload")?,
        ) {
            Ok(payload) => payload,
            Err(e) => {
                return Err("Failed to parse payload");
            }
        };
        let data_key = data
            .get("data_key")
            .ok_or("Failed to get data_key")?
            .clone();
        let title = data.get("title").ok_or("Failed to get title")?.clone();
        let next_task = match data.get("next_task") {
            None => None,
            Some(next_task) => {
                let next_task: HashMap<String, String> =
                    match serde_json::from_str(next_task) {
                        Ok(next_task) => next_task,
                        Err(e) => {
                            return Err("Failed to parse next_task");
                        }
                    };
                Some(Box::new(TaskTemplate::try_from(next_task)?))
            }
        };

        Ok(TaskTemplate {
            url,
            payload,
            data_key,
            title,
            next_task,
        })
    }
}

impl TryFrom<HashMap<String, String>> for Task {
    type Error = &'static str;

    fn try_from(data: HashMap<String, String>) -> Result<Self, Self::Error> {
        let id = data.get("id").ok_or("Failed to get id")?.parse().unwrap();
        let key = generate_task_key(id);
        let title = data.get("title").unwrap_or(&"".to_string()).clone();
        let url = data.get("url").ok_or("Failed to get url")?.clone();
        let payload = match serde_json::from_str::<serde_json::Value>(
            data.get("payload").ok_or("Failed to get payload")?,
        ) {
            Ok(payload) => payload,
            Err(e) => {
                return Err("Failed to parse payload");
            }
        };
        let data_key = data
            .get("data_key")
            .ok_or("Failed to get data_key")?
            .clone();

        Ok(Task {
            key,
            id,
            title,
            url,
            payload,
            data_key,
            status: TaskStatus::from(data["status"].clone()),
            last_updated: match data.get("last_updated") {
                None => chrono::Utc::now(),
                Some(x) => match chrono::DateTime::parse_from_rfc3339(x) {
                    Ok(timestamp) => timestamp.with_timezone(&chrono::Utc),
                    Err(e) => {
                        tracing::error!(
                            "Failed to parse last_updated timestamp: {}",
                            e
                        );
                        chrono::Utc::now()
                    }
                },
            },
            run_after: match data.get("run_after") {
                None => chrono::Utc::now(),
                Some(x) => match chrono::DateTime::parse_from_rfc3339(x) {
                    Ok(timestamp) => timestamp.with_timezone(&chrono::Utc),
                    Err(e) => {
                        tracing::error!(
                            "Failed to parse run_after timestamp: {}",
                            e
                        );
                        chrono::Utc::now()
                    }
                },
            },

            next_task: match data.get("next_task") {
                None => None,
                Some(next_task) => {
                    let next_task: HashMap<String, String> =
                        serde_json::from_str(next_task).unwrap();
                    Some(TaskTemplate::try_from(next_task).unwrap())
                }
            },
        })
    }
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
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

fn generate_task_key(id: u64) -> String {
    format!("task:item:{}", id)
}

fn generate_task_data_key(id: u64) -> String {
    format!("task:data:{}", id)
}

fn generate_temp_queue_name(queue_name: &str) -> String {
    format!("{}:temp", queue_name)
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

    let now = chrono::Utc::now();

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
            .arg(&now.to_rfc3339())
            .arg("run_after")
            .arg(&now.to_rfc3339()),
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
            run_after: now,
        }),
        Err(e) => Err(e),
    }
}

pub fn publish_task_status(
    con: &mut redis::Connection,
    task: &Task,
    previous_status: TaskStatus,
) -> Result<(), &'static str> {
    let data = json!({
       "task": task,
       "previous_status": previous_status.as_str(),
       "new_status": task.status.as_str(),
       "event": "task_status_change",
    });

    let message = match serde_json::to_string(&data) {
        Ok(message) => message,
        Err(e) => {
            tracing::error!("Failed to serialize task status message: {}", e);
            return Err("Failed to serialize task status message");
        }
    };

    con.publish("task", message).map_err(|_| {
        tracing::error!("Failed to publish task status message");
        "Failed to publish task status message"
    })
}

pub fn update_task_status(
    con: &mut redis::Connection,
    task: &Task,
    new_task_status: TaskStatus,
) -> Result<(), &'static str> {
    let now = chrono::Utc::now();
    match con.hset::<_, _, _, ()>(
        task.key.clone(),
        "status",
        new_task_status.as_str(),
    ) {
        Ok(_) => (),
        Err(e) => {
            tracing::error!("Failed to update task status: {}", e);
            return Err("Failed to update task status");
        }
    };

    match con.hset::<_, _, _, ()>(
        task.key.clone(),
        "last_updated",
        &now.to_rfc3339(),
    ) {
        Ok(_) => (),
        Err(e) => {
            tracing::error!("Failed to update task last_updated: {}", e);
            return Err("Failed to update task last_updated");
        }
    };

    if task.status != new_task_status {
        publish_task_status(
            con,
            &Task {
                key: task.key.clone(),
                id: task.id,
                title: task.title.clone(),
                url: task.url.clone(),
                payload: task.payload.clone(),
                data_key: task.data_key.clone(),
                status: new_task_status,
                last_updated: now,
                run_after: task.run_after,
            },
            task.status.clone(),
        )
    } else {
        Ok(())
    }
}

pub fn pop_task(
    con: &mut redis::Connection,
    queue_name: &str,
    retry_delay: std::time::Duration,
) -> Result<Task, &'static str> {
    let temp_queue_name = generate_temp_queue_name(queue_name);

    let task_key = loop {
        // Pop the highest priority task key from the queue
        let (task_key, score): (String, f64) =
            match con
                .bzpopmin::<&str, (String, String, String)>(queue_name, 0.0)
            {
                Ok((_, task_key, score)) => (task_key, score.parse().unwrap()),

                Err(e) => {
                    tracing::error!("Failed to pop task from queue: {}", e);
                    return Err("Failed to pop task from queue");
                }
            };

        // Check if the task's run_after timestamp is in the future
        let run_after = match chrono::DateTime::from_timestamp(score as i64, 0)
        {
            Some(timestamp) => timestamp,
            None => return Err("Failed to parse score as timestamp"),
        };
        // If it is, put the task back in the queue, wait for some time and try again
        if run_after > chrono::Utc::now() {
            match con.zadd::<&str, f64, &std::string::String, ()>(
                queue_name, &task_key, score,
            ) {
                Ok(_) => (),
                Err(_) => return Err("Failed to put task back in queue"),
            };

            // Sleep for a while
            std::thread::sleep(retry_delay);

            continue;
        }

        // If it isn't in the future, break the loop
        break task_key;
    };

    // Move the task to the temp queue
    match con.lpush::<&str, &str, ()>(&temp_queue_name, &task_key) {
        Ok(_) => (),
        Err(_) => return Err("Failed to move task to temp queue"),
    };

    let task_data: HashMap<String, String> = match con.hgetall(&task_key) {
        Ok(task_data) => task_data,
        Err(_) => return Err("Failed to get task data"),
    };

    Ok(Task::from(task_data))
}

pub fn remove_task_from_temp_queue(
    con: &mut redis::Connection,
    queue_name: &str,
    task: &Task,
) -> Result<(), &'static str> {
    let temp_queue_name = generate_temp_queue_name(queue_name);

    match con.lrem::<&std::string::String, std::string::String, ()>(
        &temp_queue_name,
        1,
        task.key.clone(),
    ) {
        Ok(_) => Ok(()),
        Err(e) => {
            tracing::error!("Failed to remove task from temp queue: {}", e);
            Err("Failed to remove task from temp queue")
        }
    }
}

pub fn queue_task(
    con: &mut redis::Connection,
    queue_name: &str,
    task: &Task,
) -> Result<(), &'static str> {
    let score = task.run_after.timestamp() as f64;

    match con.zadd::<&str, f64, &std::string::String, ()>(
        queue_name, &task.key, score,
    ) {
        Ok(_) => Ok(()),
        Err(e) => {
            tracing::error!("Failed to add task to queue: {}", e);

            Err("Failed to add task to queue")
        }
    }
}

pub fn get_task(
    con: &mut redis::Connection,
    task_id: u64,
) -> Result<Task, &'static str> {
    let task_key = generate_task_key(task_id);
    let task_data: HashMap<String, String> = match con.hgetall(task_key) {
        Ok(task_data) => task_data,
        Err(_) => return Err("Failed to get task data"),
    };

    Ok(Task::from(task_data))
}

pub fn get_task_data(
    con: &mut redis::Connection,
    record_id: u64,
) -> Result<Vec<serde_json::Value>, &'static str> {
    // get the data list from redis if it exists
    let data_key = generate_task_data_key(record_id);

    let data: Vec<String> = match con.lrange(&data_key, 0, -1) {
        Ok(data) => data,
        Err(e) => {
            tracing::error!("Failed to get data list: {}", e);
            return Err("Failed to get data list");
        }
    };

    // parse the JSON list in each item in the data list
    let data: Vec<serde_json::Value> = data
        .iter()
        .flat_map(|item| {
            serde_json::from_str::<Vec<serde_json::Value>>(item)
                .unwrap_or_default()
        })
        .collect();

    Ok(data)
}
