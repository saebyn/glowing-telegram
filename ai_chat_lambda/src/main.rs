/**
 * This is the main entry point for the `ai_chat_lambda` lambda
 *  function.
 *
 * This lambda function is responsible for handling the chat
 * requests from the user and responding with the appropriate
 * response. It uses the `openai_dive` library to interact with
 * the `OpenAI` API.
 */
use aws_config::{BehaviorVersion, meta::region::RegionProviderChain};

use axum::{
    Json, Router,
    body::Body,
    extract::State,
    http::{Request, StatusCode, header},
    response::IntoResponse,
    routing::post,
};
use figment::Figment;
use lambda_http::tower;
use openai_dive::v1::error::APIError;
use openai_dive::v1::resources::chat::{
    ChatCompletionParameters, ChatCompletionResponse,
};
use openai_dive::v1::{
    api::Client,
    resources::chat::{
        ChatCompletionResponseFormat, ChatMessage, ChatMessageContent,
    },
};
use serde::{Deserialize, Serialize};
use serde_json::json;
use tower_http::{compression::CompressionLayer, trace::TraceLayer};
use tracing::instrument;
use types::SimpleChatMessage;

#[derive(Debug, Deserialize, Clone)]
#[allow(clippy::struct_field_names)]
struct Config {
    openai_secret_arn: String,
    openai_model: String,
}

fn load_config() -> Result<Config, figment::Error> {
    let figment = Figment::new().merge(figment::providers::Env::raw());

    figment.extract()
}

#[derive(Deserialize, Debug)]
struct ChatRequest {
    messages: Vec<SimpleChatMessage>,
}

#[derive(Serialize)]
struct ChatResponse {
    messages: Vec<SimpleChatMessage>,
}

#[derive(Debug, Clone)]
struct AppState {
    secrets_manager: aws_sdk_secretsmanager::Client,
    config: Config,
}

#[tokio::main]
async fn main() {
    // https://docs.aws.amazon.com/lambda/latest/dg/rust-logging.html
    tracing_subscriber::fmt()
        .json()
        // allow log level to be overridden by RUST_LOG env var
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        // this needs to be set to remove duplicated information in the log.
        .with_current_span(false)
        // this needs to be set to false, otherwise ANSI color codes will
        // show up in a confusing manner in CloudWatch logs.
        .with_ansi(false)
        // disabling time is handy because CloudWatch will add the ingestion time.
        .without_time()
        // remove the name of the function from every log entry
        .with_target(false)
        .init();

    let config = load_config().expect("failed to load config");
    let region_provider =
        RegionProviderChain::default_provider().or_else("us-east-1");
    let aws_config = aws_config::defaults(BehaviorVersion::latest())
        .region(region_provider)
        .load()
        .await;

    let secrets_manager = aws_sdk_secretsmanager::Client::new(&aws_config);

    let state = AppState {
        secrets_manager,
        config,
    };

    // Set up a trace layer
    let trace_layer = TraceLayer::new_for_http().on_request(
        |request: &Request<Body>, _: &tracing::Span| {
            tracing::info!(
                "received request: {method} {uri}",
                method = request.method(),
                uri = request.uri()
            );
        },
    );

    let compression_layer = CompressionLayer::new().gzip(true).deflate(true);

    let app = Router::new()
        .route("/ai/chat", post(handler))
        .fallback(|| async {
            (
                StatusCode::NOT_FOUND,
                [(header::CONTENT_TYPE, "application/json")],
                Json(json!({
                    "message": "not found",
                })),
            )
        })
        .layer(trace_layer)
        .layer(compression_layer)
        .with_state(state);

    // Provide the app to the lambda runtime
    let app = tower::ServiceBuilder::new()
        .layer(axum_aws_lambda::LambdaLayer::default().trim_stage())
        .service(app);

    lambda_http::run(app).await.unwrap();
}

#[instrument(skip(state))]
async fn handler(
    State(state): State<AppState>,
    Json(event): Json<ChatRequest>,
) -> impl IntoResponse {
    // Get the openai api key from secrets manager
    let openai_secret = match state
        .secrets_manager
        .get_secret_value()
        .secret_id(&state.config.openai_secret_arn)
        .send()
        .await
    {
        Ok(secret) => secret,
        Err(e) => {
            tracing::error!("failed to get secret: {:?}", e);
            return (StatusCode::INTERNAL_SERVER_ERROR,).into_response();
        }
    };

    let openai_secret =
        match openai_secret.secret_string.ok_or("Secret string not found") {
            Ok(secret) => secret,
            Err(e) => {
                tracing::error!("failed to get secret string: {:?}", e);
                return (StatusCode::INTERNAL_SERVER_ERROR,).into_response();
            }
        };

    let client = Client::new(openai_secret);

    let response = match client
        .chat()
        .create(build_parameters(
            &event.messages,
            &state.config.openai_model,
        ))
        .await
    {
        Ok(response) => response,
        Err(e) => {
            tracing::error!("Failed to complete chat: {:?}", e);
            match e {
                APIError::InvalidRequestError(message) => {
                    tracing::error!("Invalid request: {:?}", message);
                    return (StatusCode::INTERNAL_SERVER_ERROR,)
                        .into_response();
                }
                _ => {
                    return (StatusCode::INTERNAL_SERVER_ERROR,)
                        .into_response();
                }
            }
        }
    };

    let response = ChatResponse {
        messages: event
            .messages
            .into_iter()
            .chain(std::iter::once(convert_chat_completion(&response)))
            .collect(),
    };

    (
        StatusCode::OK,
        [(header::CONTENT_TYPE, "application/json")],
        Json(response),
    )
        .into_response()
}

fn build_parameters(
    messages: &[SimpleChatMessage],
    model: &str,
) -> ChatCompletionParameters {
    ChatCompletionParameters {
        model: model.to_string(),
        response_format: Some(ChatCompletionResponseFormat::JsonObject),
        messages: messages
            .iter()
            .map(|m| match m.role {
                types::Role::System => ChatMessage::System {
                    name: None,
                    content: ChatMessageContent::Text(m.content.clone()),
                },
                _ => ChatMessage::User {
                    name: None,
                    content: ChatMessageContent::Text(m.content.clone()),
                },
            })
            .collect(),
        ..Default::default()
    }
}

fn convert_chat_completion(
    response: &ChatCompletionResponse,
) -> SimpleChatMessage {
    // TODO if there is more content to be generated, we will get it here
    match &response.choices[0].finish_reason {
        Some(reason) => {
            tracing::info!("Finish reason: {:?}", reason);
        }
        None => {
            tracing::info!("No finish reason provided");
        }
    }

    let (role, content) = match &response.choices[0].message {
        ChatMessage::User { content, .. } => ("user", Some(content.clone())),
        ChatMessage::System { content, .. } => {
            ("system", Some(content.clone()))
        }
        ChatMessage::Assistant { content, .. } => {
            ("assistant", content.clone())
        }
        ChatMessage::Developer { content, .. } => {
            ("developer", Some(content.clone()))
        }
        ChatMessage::Tool { content, .. } => {
            ("tool", Some(ChatMessageContent::Text(content.to_string())))
        }
    };

    SimpleChatMessage {
        content: match content {
            None
            | Some(
                openai_dive::v1::resources::chat::ChatMessageContent::None,
            ) => String::new(),

            Some(ChatMessageContent::Text(text)) => text,

            Some(ChatMessageContent::ContentPart(content)) => {
                format!("{content:?}")
            }
        },

        role: match role {
            "user" => types::Role::User,
            "assistant" => types::Role::Assistant,
            "tool" => types::Role::Tool,
            _ => types::Role::System,
        },
    }
}
