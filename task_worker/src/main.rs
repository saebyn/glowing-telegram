use redis::Commands;
use tracing::instrument;
use tracing_subscriber::prelude::*;

use task_worker::{
    pop_task, queue_task, remove_task_from_temp_queue, update_task_status, TaskStatus,
};

const DEFAULT_RETRY_DELAY: chrono::TimeDelta = chrono::Duration::minutes(1);
const NO_TASK_READY_DELAY: std::time::Duration = std::time::Duration::from_secs(60);

#[tokio::main]
async fn main() {
    println!("Starting task worker");

    let fmt_layer = tracing_subscriber::fmt::layer();

    tracing_subscriber::registry()
        .with(fmt_layer)
        .with(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    let client = reqwest::Client::new();

    let mut con = redis::Client::open(dotenvy::var("REDIS_URL").expect("REDIS_URL must be set"))
        .expect("Failed to open redis client")
        .get_connection()
        .expect("Failed to get redis connection");

    let queue_name = dotenvy::var("QUEUE_NAME").expect("QUEUE_NAME must be set");

    loop {
        work(&client, &mut con, &queue_name).await;
    }
}

#[instrument(skip(con))]
async fn work(reqwest_client: &reqwest::Client, con: &mut redis::Connection, queue_name: &str) {
    /*
     * Take a task from the queue and then while the target url returns a
     * cursor, store the data from the data_key into the task data as a
     * json string of an array and call the target url with the cursor
     * until the cursor is null.
     */
    let mut task =
        pop_task(con, queue_name, NO_TASK_READY_DELAY).expect("Failed to pop task");

    // update the status to processing
    update_task_status(con, &task, TaskStatus::Processing)
        .expect("Failed to update task status");

    // loop while the cursor is not Null
    loop {
        let response = reqwest_client
            .post(&task.url)
            .json(&task.payload)
            .send()
            .await
            .expect("Failed to get response from url");

        // if the repsonse is a 503 Service Unavailable, then mark the
        // task as queued again, add a retry timestamp, and break
        if response.status() == reqwest::StatusCode::SERVICE_UNAVAILABLE {
            update_task_status(con, &task, TaskStatus::Queued)
                .expect("Failed to update task status");

            // add retry timestamp to the task payload
            let run_after = response
                .headers()
                .get("Retry-After")
                .map(|header| header.to_str().unwrap_or(""))
                .unwrap_or("");

            task.run_after = match run_after.parse::<u64>() {
                Ok(timestamp) => chrono::DateTime::from_timestamp(
                    chrono::Utc::now().timestamp() + timestamp as i64,
                    0,
                )
                .expect("Failed to create timestamp"),
                Err(_) => match run_after.parse::<chrono::DateTime<chrono::Utc>>() {
                    Ok(timestamp) => timestamp,
                    Err(e) => {
                        tracing::info!("Failed to parse Retry-After header: {}", e);
                        chrono::Utc::now() + DEFAULT_RETRY_DELAY
                    }
                },
            };

            // put the task id in the main queue
            queue_task(con, queue_name, &task).expect("Failed to add task to queue");

            // remove the task from the temp queue
            remove_task_from_temp_queue(con, queue_name, &task)
                .expect("Failed to remove task from temp queue");

            break;
        }

        // if the response is not 200, then update the status to failed and break
        if !response.status().is_success() {
            update_task_status(con, &task, TaskStatus::Failed)
                .expect("Failed to update task status");

            break;
        }

        let response = response
            .json::<serde_json::Value>()
            .await
            .expect("Failed to parse response as json");

        println!("Got response: {:?}", response);

        let cursor = &response["cursor"];

        // Iterate using the returned cursor

        // Store the data from the data_key into the task data as a json string of an array
        let data = &response[task.data_key.as_str()];

        let data_str = serde_json::to_string(&data).expect("Failed to serialize data as json");

        let task_data_key = format!("task:data:{}", task.id);
        // TODO move this to a function called save_task_data
        let _: () = con
            .rpush(&task_data_key, data_str)
            .expect("Failed to save task data");

        if cursor.is_null() {
            break;
        }

        task.payload["cursor"] = cursor.clone();
    }

    println!("Finished task: {}", task.key);

    // update the status to complete
    update_task_status(con, &task, TaskStatus::Complete)
        .expect("Failed to update task status");

    // remove the task from redis
    remove_task_from_temp_queue(con, queue_name, &task)
        .expect("Failed to remove task from temp queue");
}
