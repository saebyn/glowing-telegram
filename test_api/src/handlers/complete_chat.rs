use axum::extract::State;
use axum::{http::StatusCode, response::IntoResponse, Json};
use openai_dive::v1::api::Client;
use openai_dive::v1::resources::chat_completion::{ChatCompletionParameters, ChatMessage, Role};
use serde::{Deserialize, Serialize};

use crate::state::AppState;

pub async fn handler(
    State(state): State<AppState>,
    // this argument tells axum to parse the request body
    // as JSON into a list of `ChatMessage` type records
    Json(payload): Json<Vec<SimpleChatMessage>>,
) -> impl IntoResponse {
    let client = Client::new(state.openai_key());

    let parameters = ChatCompletionParameters {
        model: "gpt-4".to_string(),
        messages: payload
            .iter()
            .map(|m| {
                // Role::from_str does not exist, so we have to do this
                let role = match m.role.as_str() {
                    "system" => Role::System,
                    "assistant" => Role::Assistant,
                    "user" => Role::User,
                    _ => Role::User,
                };

                return ChatMessage {
                    content: m.content.clone(),
                    role: role,
                    name: None,
                };
            })
            .collect(),
        ..Default::default()
    };

    let response = client
        .chat()
        .create(parameters)
        .await
        .expect("failed to complete chat");

    let message = SimpleChatMessage {
        content: response.choices[0].message.content.clone(),
        role: response.choices[0].message.role.to_string().to_lowercase(),
    };

    // take the original payload and add message to the end
    let mut payload = payload;
    payload.push(message);

    // this will be converted into a JSON response
    // with a status code of `200 OK`
    (StatusCode::OK, Json(payload))
}

// the input to our `complete_chat` handler
#[derive(Deserialize, Serialize, Debug)]
pub struct SimpleChatMessage {
    content: String,
    // TODO how do we make this require specific values?
    role: String,
}
