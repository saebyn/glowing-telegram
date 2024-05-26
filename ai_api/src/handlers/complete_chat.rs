use axum::extract::State;
use axum::{http::StatusCode, response::IntoResponse, Json};
use openai_dive::v1::api::Client;
use openai_dive::v1::resources::chat::{
    ChatCompletionParameters, ChatCompletionResponseFormat, ChatCompletionResponseFormatType,
    ChatMessage, ChatMessageContent, Role,
};
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
        model: state.openai_model(),
        response_format: Some(ChatCompletionResponseFormat {
            r#type: ChatCompletionResponseFormatType::JsonObject,
        }),
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

                ChatMessage {
                    content: ChatMessageContent::Text(m.content.clone()),
                    role,
                    name: None,
                    ..Default::default()
                }
            })
            .collect(),
        ..Default::default()
    };

    let response = client
        .chat()
        .create(parameters)
        .await
        .expect("failed to complete chat");

    // log the whole response
    tracing::info!("{:?}", response);

    // take the first choice and convert it into a `SimpleChatMessage`
    let message = SimpleChatMessage {
        content: match &response.choices[0].message.content {
            ChatMessageContent::Text(text) => text.to_string(),
            _ => "No text content".to_string(),
        },
        role: response.choices[0].message.role.to_string().to_lowercase(),
    };

    // if there is more content to be generated, we will get it here
    match &response.choices[0].finish_reason {
        Some(reason) => {
            tracing::info!("Finish reason: {:?}", reason);
        }
        None => {
            tracing::info!("No finish reason provided");
        }
    }

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
