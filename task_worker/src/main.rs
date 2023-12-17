use std::collections::HashMap;

use redis::Commands;
use serde_json::json;

#[tokio::main]
async fn main() {
    println!("Starting task worker");

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

        // get the payload from the task data
        let payload_str = task_data
            .get("payload")
            .expect("Failed to get payload from task data");
        // parse the payload as json
        let mut payload_json: serde_json::Value =
            serde_json::from_str(payload_str).expect("Failed to parse payload as json");

        //payload_json["cursor"] = json!("hi");

        let client = reqwest::Client::new();

        let response = client
            .post(&task_data["url"])
            .json(&payload_json)
            .send()
            .await
            .expect("Failed to get response from url")
            .json::<serde_json::Value>()
            .await
            .expect("Failed to parse response as json");

        println!("Got response: {:?}", response);

        // TODO Iterate using the returned cursor

        // TODO Store the data from the data_key into the task data as a json string of an array in the data field

        println!("Finished task: {}", task_key);

        let _: () = con
            .lrem(&temp_queue_name, 1, task_key)
            .expect("Failed to remove task from temp queue");
    }
}
