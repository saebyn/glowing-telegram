use axum::{http::StatusCode, response::IntoResponse, routing::post, Json, Router};
use openai_dive::v1::api::Client;
use openai_dive::v1::resources::chat_completion::{ChatCompletionParameters, ChatMessage, Role};
use serde::{Deserialize, Serialize};
use std::net::SocketAddr;
use tower_http::cors::CorsLayer;

#[tokio::main]
async fn main() {
    // initialize tracing
    tracing_subscriber::fmt::init();

    // TODO how to read in context/settings and share them with the route handlers?

    // build our application with a route
    let app = Router::new()
        // `POST /api/chat` goes to `complete_chat`
        .route("/api/chat", post(complete_chat))
        // TODO how do we make this only allow requests from our frontend?
        .layer(CorsLayer::permissive());

    // run our app with hyper
    let addr = SocketAddr::from(([127, 0, 0, 1], 3000));
    tracing::debug!("listening on {}", addr);
    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .await
        .unwrap();
}

async fn complete_chat(
    // this argument tells axum to parse the request body
    // as JSON into a list of `ChatMessage` type records
    Json(payload): Json<Vec<SimpleChatMessage>>,
) -> impl IntoResponse {
    // insert your application logic here
    // read openai api key from from "../openai_key.txt"
    let openai_key = std::fs::read_to_string("../openai_key.txt")
        .expect("failed to read openai_key.txt")
        .trim()
        .to_string();

    let client = Client::new(openai_key);

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
#[derive(Deserialize, Serialize)]
struct SimpleChatMessage {
    content: String,
    // TODO how do we make this require specific values?
    role: String,
}
