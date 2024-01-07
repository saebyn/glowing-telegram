use std::collections::HashMap;

use redis::Commands;

#[tokio::main]
async fn main() {
    println!("Starting task worker");

    let client = reqwest::Client::new();

    let mut con = redis::Client::open(dotenvy::var("REDIS_URL").expect("REDIS_URL must be set"))
        .expect("Failed to open redis client")
        .get_connection()
        .expect("Failed to get redis connection");

    let queue_name = dotenvy::var("QUEUE_NAME").expect("QUEUE_NAME must be set");

    let temp_queue_name = format!("{}:temp", queue_name);

    loop {
        let task_key: String = con
            .blmove(
                &queue_name,
                &temp_queue_name,
                redis::Direction::Right,
                redis::Direction::Left,
                0.0,
            )
            .expect("Failed to get task from queue");

        println!("Got task key: {}", task_key);

        // get the task data
        let task_data: HashMap<String, String> = con
            .hgetall(&task_key)
            .expect("Failed to get task data from redis");

        println!("Got task data: {:?}", task_data);

        // update the status to processing
        let _: () = con
            .hset(&task_key, "status", "processing")
            .expect("Failed to update task status");

        // get the payload from the task data
        let payload_str = task_data
            .get("payload")
            .expect("Failed to get payload from task data");
        // parse the payload as json
        let mut payload_json: serde_json::Value =
            serde_json::from_str(payload_str).expect("Failed to parse payload as json");

        let data_key = task_data
            .get("data_key")
            .expect("Failed to get data_key from task data");

        // loop while the cursor is not Null
        loop {
            let response = client
                .post(&task_data["url"])
                .json(&payload_json)
                .send()
                .await
                .expect("Failed to get response from url");

            // if the response is not 200, then update the status to failed and break
            if !response.status().is_success() {
                let _: () = con
                    .hset(&task_key, "status", "failed")
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
            let data = &response[data_key];

            let data_str = serde_json::to_string(&data).expect("Failed to serialize data as json");

            let task_data_key = format!("{}:data", task_key);

            let _: () = con
                .rpush(&task_data_key, data_str)
                .expect("Failed to save task data");

            if cursor.is_null() {
                break;
            }

            payload_json["cursor"] = cursor.clone();
        }

        println!("Finished task: {}", task_key);

        // update the status to complete
        let _: () = con
            .hset(&task_key, "status", "complete")
            .expect("Failed to update task status");

        let _: () = con
            .lrem(&temp_queue_name, 1, task_key)
            .expect("Failed to remove task from temp queue");
    }
}
