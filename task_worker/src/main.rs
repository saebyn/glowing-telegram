use redis::Commands;

use task_worker::{pop_task, remove_task_from_temp_queue, update_task_status, TaskStatus};

#[tokio::main]
async fn main() {
    println!("Starting task worker");

    let client = reqwest::Client::new();

    let mut con = redis::Client::open(dotenvy::var("REDIS_URL").expect("REDIS_URL must be set"))
        .expect("Failed to open redis client")
        .get_connection()
        .expect("Failed to get redis connection");

    let queue_name = dotenvy::var("QUEUE_NAME").expect("QUEUE_NAME must be set");

    loop {
        let mut task = pop_task(&mut con, &queue_name);

        // update the status to processing
        update_task_status(&mut con, &task, TaskStatus::Processing);

        // loop while the cursor is not Null
        loop {
            let response = client
                .post(&task.url)
                .json(&task.payload)
                .send()
                .await
                .expect("Failed to get response from url");

            // if the response is not 200, then update the status to failed and break
            if !response.status().is_success() {
                update_task_status(&mut con, &task, TaskStatus::Failed);

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

            let task_data_key = format!("task:data:{}", task_data["id"].as_str());

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
        update_task_status(&mut con, &task, TaskStatus::Complete);

        // remove the task from redis
        remove_task_from_temp_queue(&mut con, &task);
    }
}
