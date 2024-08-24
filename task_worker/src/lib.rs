use redis::{Commands, ConnectionLike};
use serde::{Deserialize, Serialize};
use serde_json::json;
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

/**
 * A payload transformer is a struct that contains a destination key and a
 * source JSONPath.
 *
 * The source JSONPath is used to extract a value from the payload and assign it
 * to the destination key in the transformed payload.
 */
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct PayloadTransform {
    pub destination_key: String,
    pub source_pointer: String,
}

fn serialize_method<S>(
    method: &reqwest::Method,
    serializer: S,
) -> Result<S::Ok, S::Error>
where
    S: serde::Serializer,
{
    serializer.serialize_str(method.as_str())
}

fn deserialize_method<'de, D>(
    deserializer: D,
) -> Result<reqwest::Method, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let method_str = String::deserialize(deserializer)?;
    reqwest::Method::from_bytes(method_str.as_bytes())
        .map_err(serde::de::Error::custom)
}

fn default_http_method() -> reqwest::Method {
    reqwest::Method::POST
}

#[derive(Deserialize, Serialize, Debug)]
pub struct TaskRequest {
    pub url: String,
    pub payload: serde_json::Value,
    pub title: String,
    pub data_key: String,

    pub next_task: Option<TaskTemplate>,

    #[serde(
        serialize_with = "serialize_method",
        deserialize_with = "deserialize_method",
        default = "default_http_method"
    )]
    pub http_method: reqwest::Method,

    pub payload_transformer: Option<Vec<PayloadTransform>>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Task {
    // TODO consider removing this and calculating from id
    pub key: String,
    pub id: u64,
    pub previous_task_id: Option<u64>,
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

    #[serde(
        serialize_with = "serialize_method",
        deserialize_with = "deserialize_method",
        default = "default_http_method"
    )]
    pub http_method: reqwest::Method,

    pub payload_transformer: Option<Vec<PayloadTransform>>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct TaskTemplate {
    pub url: String,
    pub payload: serde_json::Value,
    pub data_key: String,
    pub title: String,
    pub next_task: Option<Box<TaskTemplate>>,

    #[serde(
        serialize_with = "serialize_method",
        deserialize_with = "deserialize_method"
    )]
    pub http_method: reqwest::Method,

    /**
     * A list of payload transformers to apply to the payload before sending
     * the request.
     */
    pub payload_transformer: Option<Vec<PayloadTransform>>,
}

const TASK_COUNTER_KEY: &str = "task:counter";

impl TryFrom<HashMap<String, String>> for TaskTemplate {
    type Error = &'static str;

    fn try_from(data: HashMap<String, String>) -> Result<Self, Self::Error> {
        let url = data.get("url").ok_or("Failed to get url")?.clone();
        let payload = match serde_json::from_str::<serde_json::Value>(
            data.get("payload").ok_or("Failed to get payload")?,
        ) {
            Ok(payload) => payload,
            Err(e) => {
                tracing::error!("Failed to parse payload: {}", e);
                return Err("Failed to parse payload");
            }
        };
        let data_key = data
            .get("data_key")
            .ok_or("Failed to get data_key")?
            .clone();
        let title = data.get("title").ok_or("Failed to get title")?.clone();

        let next_task = match data.get("next_task").map(|x| x.as_str()) {
            None => None,
            Some("null") => None,
            Some(next_task) => {
                let next_task: HashMap<String, String> =
                    match serde_json::from_str(next_task) {
                        Ok(next_task) => next_task,
                        Err(e) => {
                            tracing::error!(
                                "Failed to parse next_task: {}",
                                e
                            );
                            return Err("Failed to parse next_task");
                        }
                    };
                Some(Box::new(TaskTemplate::try_from(next_task)?))
            }
        };

        let http_method = deserialize_http_method(&data);

        let payload_transformer = deserialize_payload_transformer(&data);

        Ok(TaskTemplate {
            url,
            payload,
            data_key,
            title,
            next_task,

            http_method,
            payload_transformer,
        })
    }
}

fn deserialize_http_method(data: &HashMap<String, String>) -> reqwest::Method {
    match data.get("http_method") {
        None => reqwest::Method::POST,
        Some(http_method) => {
            match reqwest::Method::from_bytes(http_method.as_bytes()) {
                Ok(http_method) => http_method,
                Err(e) => {
                    tracing::error!("Failed to parse http_method: {}", e);
                    reqwest::Method::POST
                }
            }
        }
    }
}

fn deserialize_payload_transformer(
    data: &HashMap<String, String>,
) -> Option<Vec<PayloadTransform>> {
    let payload_transformer = match data.get("payload_transformer") {
        None => None,
        Some(payload_transformer) => {
            if payload_transformer == "null" {
                None
            } else {
                match serde_json::from_str::<Vec<PayloadTransform>>(
                    payload_transformer,
                ) {
                    Ok(payload_transformer) => Some(payload_transformer),
                    Err(e) => {
                        tracing::error!(
                            "Failed to parse payload_transformer: {}",
                            e
                        );
                        None
                    }
                }
            }
        }
    };
    payload_transformer
}

impl TryFrom<&Task> for redis::Cmd {
    type Error = &'static str;

    fn try_from(task: &Task) -> Result<Self, Self::Error> {
        let next_task = match serde_json::to_string(&task.next_task) {
            Ok(next_task) => next_task,
            Err(e) => {
                tracing::error!("Failed to serialize next_task: {}", e);
                return Err("Failed to serialize next_task");
            }
        };

        let previous_task_id = json!(task.previous_task_id).to_string();

        Ok(redis::cmd("HMSET")
            .arg(&task.key)
            .arg("id")
            .arg(task.id)
            .arg("title")
            .arg(&task.title)
            .arg("status")
            .arg(task.status.as_str())
            .arg("url")
            .arg(&task.url)
            .arg("payload")
            .arg(task.payload.to_string())
            .arg("data_key")
            .arg(&task.data_key)
            .arg("last_updated")
            .arg(task.last_updated.to_rfc3339())
            .arg("run_after")
            .arg(task.run_after.to_rfc3339())
            .arg("next_task")
            .arg(next_task)
            .arg("previous_task_id")
            .arg(previous_task_id)
            .arg("http_method")
            .arg(task.http_method.as_str())
            .arg("payload_transformer")
            .arg(serde_json::to_string(&task.payload_transformer).unwrap())
            .to_owned())
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
            Err(_) => {
                return Err("Failed to parse payload");
            }
        };
        let data_key = data
            .get("data_key")
            .ok_or("Failed to get data_key")?
            .clone();

        let previous_task_id = match data.get("previous_task_id") {
            None => None,
            Some(previous_task_id) => {
                if previous_task_id == "null" {
                    None
                } else {
                    match previous_task_id.parse::<u64>() {
                        Ok(previous_task_id) => Some(previous_task_id),
                        Err(e) => {
                            tracing::error!("Failed to parse previous_task_id: {}. Value was {}", e, previous_task_id);
                            None
                        }
                    }
                }
            }
        };

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
                    match serde_json::from_str::<TaskTemplate>(next_task) {
                        Err(_) => None,
                        Ok(next_task) => Some(next_task),
                    }
                }
            },

            http_method: deserialize_http_method(&data),
            payload_transformer: deserialize_payload_transformer(&data),

            previous_task_id,
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
    title: &str,
    url: &str,
    payload: serde_json::Value,
    data_key: &str,
    next_task: Option<TaskTemplate>,
    previous_task_id: Option<u64>,
    http_method: reqwest::Method,
    payload_transformer: Option<Vec<PayloadTransform>>,
) -> Result<Task, &'static str> {
    // generate a unique id for the task by incrementing the task counter
    let id: u64 = match con.incr(TASK_COUNTER_KEY, 1) {
        Ok(id) => {
            tracing::info!("Generated task id: {}", id);
            id
        }
        Err(e) => {
            tracing::error!("Failed to generate task id: {}", e);
            return Err("Failed to generate task id");
        }
    };

    let key = generate_task_key(id);

    let now = chrono::Utc::now();

    let task = Task {
        key,
        id,
        title: title.to_string(),
        url: url.to_string(),
        payload,
        data_key: data_key.to_string(),
        status: TaskStatus::Queued,
        last_updated: now,
        run_after: now,
        next_task,
        previous_task_id,
        http_method,
        payload_transformer,
    };

    match con.req_command(&TryFrom::try_from(&task)?) {
        Ok(_) => Ok(task),
        Err(e) => {
            tracing::error!("Failed to create task: {}", e);
            Err("Failed to create task")
        }
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
        let mut task = task.clone();

        task.status = new_task_status;
        task.last_updated = now;

        publish_task_status(con, &task, task.status.clone())
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

    Task::try_from(task_data)
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

/**
 * Add a task to the queue
 *
 * The task must already exist in Redis before it can be added to the queue.
 * This is done via the `create_task` function.
 */
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

    Task::try_from(task_data)
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

    let data = combine_data(data)?;

    Ok(data)
}

/**
 * Combine a list of JSON strings representing arrays into a single array
 */
fn combine_data(
    data: Vec<String>,
) -> Result<Vec<serde_json::Value>, &'static str> {
    let data: Vec<serde_json::Value> = data
        .iter()
        .map(|item| serde_json::from_str::<Vec<serde_json::Value>>(item))
        .try_fold(vec![], |mut acc, item| {
            item.map(|item| {
                acc.extend(item);
                acc
            })
        })
        .map_err(|e| {
            tracing::error!("Failed to parse data list: {}", e);
            "Failed to parse data list"
        })?;
    Ok(data)
}

pub fn build_task_payload(
    con: &mut redis::Connection,
    task: &Task,
) -> serde_json::Value {
    let mut payload = task.payload.clone();

    // if task.previous_task_id is set, get the data from the previous task
    // by retrieving the data_key from the previous task and getting the data
    if let Some(previous_task_id) = task.previous_task_id {
        let data = match get_task_data(con, previous_task_id) {
            Ok(data) => data,
            Err(_) => {
                tracing::error!("Failed to get data from previous task");
                return payload;
            }
        };

        payload["@previous_task_data"] = data.into();
    }

    // apply payload transformers
    if let Some(payload_transformer) = &task.payload_transformer {
        apply_payload_transformers(&payload, payload_transformer)
    } else {
        payload
    }
}

fn apply_payload_transformers(
    payload: &serde_json::Value,
    payload_transformer: &Vec<PayloadTransform>,
) -> serde_json::Value {
    let mut transformed_payload = json!({});

    for transformer in payload_transformer {
        let value = payload
            .pointer(transformer.source_pointer.as_str())
            .expect("Failed to get value from payload")
            .clone();

        transformed_payload[&transformer.destination_key] = value.clone();
    }

    transformed_payload
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_task_key() {
        let id = 1;
        let key = generate_task_key(id);
        assert_eq!(key, "task:item:1");
    }

    #[test]
    fn test_generate_task_data_key() {
        let id = 1;
        let key = generate_task_data_key(id);
        assert_eq!(key, "task:data:1");
    }

    #[test]
    fn test_generate_temp_queue_name() {
        let queue_name = "test";
        let temp_queue_name = generate_temp_queue_name(queue_name);
        assert_eq!(temp_queue_name, "test:temp");
    }

    #[test]
    fn test_combine_data_success() {
        let data = vec!["[1, 2, 3]".to_string(), "[4, 5, 6]".to_string()];
        let data = combine_data(data).unwrap();
        assert_eq!(data, vec![1, 2, 3, 4, 5, 6]);
    }

    #[test]
    fn test_combine_data_failure_not_array() {
        let data = vec!["\"test\"".to_string()];
        let data = combine_data(data);
        assert_eq!(data, Err("Failed to parse data list"));
    }
}
