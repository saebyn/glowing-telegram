use redis::Commands;

use task_worker::{pop_task, remove_task_from_temp_queue, update_task_status, TaskStatus, push_task, generate_task_data_key, get_next_task_id};

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

            let task_data_key = generate_task_data_key(task.id);
            let _: () = con
                .rpush(&task_data_key, data_str)
                .expect("Failed to save task data");

            if cursor.is_null() {
                break;
            }

            task.payload["cursor"] = cursor.clone();
        }

        
        // if the task has a next_task, then create a new task with the data from the previous task
        if let Some(next_task) = task.next_task {
            let task_data_key = generate_task_data_key(task.id);
            let data = con
                .lrange(&task_data_key, 0, -1)
                .expect("Failed to get task data");

            let data: Vec<String> = data;

            let mut payload = next_task.payload.clone();
            payload["data"] = serde_json::Value::Array(
                data.iter()
                    .map(|d| serde_json::from_str(d).expect("Failed to parse data"))
                    .collect(),
            );

            let next_task_id = get_next_task_id(&mut con).expect("Failed to get next task id");

            let next_task = task_worker::Task {
                id: next_task_id,
                key: generate_task_key(next_task_id),
                url: next_task.url,
                payload,
                data_key: next_task.data_key,
                next_task: next_task.next_task,
            };

            // push the next task to the queue
            push_task(&mut con, &queue_name, &next_task);
        }
        
        println!("Finished task: {}", task.key);

        // update the status to complete
        update_task_status(&mut con, &task, TaskStatus::Complete);

        // remove the task from redis
        remove_task_from_temp_queue(&mut con, &task);
    }
}
