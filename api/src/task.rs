use serde::{Deserialize, Serialize};

#[derive(Serialize, Debug)]
pub struct TaskRequest {
    pub url: String,
    pub payload: serde_json::Value,
    pub title: String,
    pub data_key: String,
}

#[derive(Deserialize, Debug)]
struct TaskResponse {
    id: String,
}

pub struct Context {
    pub http_client: reqwest::Client,
    pub task_api_url: String,
    pub task_api_external_url: String,
}

pub async fn start(context: Context, task_request: TaskRequest) -> Result<String, String> {
    let response = match context
        .http_client
        .post(&context.task_api_url)
        .json(&task_request)
        .send()
        .await
    {
        Ok(response) => response,
        Err(err) => return Err(err.to_string()),
    };

    if !response.status().is_success() {
        return Err("task api error".to_string());
    }

    let task_response = match response.json::<TaskResponse>().await {
        Ok(task_response) => task_response,
        Err(err) => return Err(err.to_string()),
    };

    Ok(format!(
        "{}/{}",
        context.task_api_external_url, task_response.id
    ))
}
