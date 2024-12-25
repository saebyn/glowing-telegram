/**
 * This is the main entry point for the `ai_chat_lambda` lambda
 *  function.
 *
 * This lambda function is responsible for handling the chat
 * requests from the user and responding with the appropriate
 * response. It uses the `openai_dive` library to interact with
 * the `OpenAI` API.
 */
use aws_config::{meta::region::RegionProviderChain, BehaviorVersion};

use figment::Figment;
use lambda_runtime::{service_fn, Error, LambdaEvent};
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

#[derive(Debug, Deserialize)]
struct Config {
    openai_secret_arn: String,
    openai_model: String,
}

fn load_config() -> Result<Config, figment::Error> {
    let figment = Figment::new().merge(figment::providers::Env::raw());

    figment.extract()
}

#[derive(Deserialize, Serialize, Debug)]
pub struct SimpleChatMessage {
    content: String,
    // TODO how do we make this require specific values?
    role: String,
}

#[derive(Deserialize, Debug)]
struct Request {
    messages: Vec<SimpleChatMessage>,
}

#[derive(Serialize)]
struct Response {
    messages: Vec<SimpleChatMessage>,
}

#[derive(Debug)]
struct SharedResources {
    secrets_manager: aws_sdk_secretsmanager::Client,
    config: Config,
}

#[tokio::main]
async fn main() -> Result<(), Error> {
    // https://docs.aws.amazon.com/lambda/latest/dg/rust-logging.html
    tracing_subscriber::fmt()
        .json()
        .with_max_level(tracing::Level::INFO)
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

    let shared_resources = &SharedResources {
        secrets_manager,
        config,
    };

    lambda_runtime::run(service_fn(
        move |event: LambdaEvent<Request>| async move {
            handler(shared_resources, event).await
        },
    ))
    .await?;

    Ok(())
}

async fn handler(
    shared_resources: &SharedResources,
    event: LambdaEvent<Request>,
) -> Result<Response, Error> {
    // Get the openai api key from secrets manager
    let openai_secret = shared_resources
        .secrets_manager
        .get_secret_value()
        .secret_id(&shared_resources.config.openai_secret_arn)
        .send()
        .await
        .expect("failed to get secret")
        .secret_string
        .expect("secret not found");

    let client = Client::new(openai_secret);

    let response = match client
        .chat()
        .create(build_parameters(
            &event.payload.messages,
            &shared_resources.config.openai_model,
        ))
        .await
    {
        Ok(response) => response,
        Err(e) => {
            tracing::error!("Failed to complete chat: {:?}", e);
            match e {
                APIError::InvalidRequestError(message) => {
                    return Err(format!(
                        "Failed to complete chat: {message:?}"
                    )
                    .into())
                }
                _ => {
                    return Err("Unexpected error occurred while processing the chat request".into());
                }
            }
        }
    };

    Ok(Response {
        messages: event
            .payload
            .messages
            .into_iter()
            .chain(std::iter::once(SimpleChatMessage::from(response)))
            .collect(),
    })
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
            .map(|m| match m.role.as_str() {
                "system" => ChatMessage::System {
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

impl From<ChatCompletionResponse> for SimpleChatMessage {
    fn from(response: ChatCompletionResponse) -> Self {
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
            ChatMessage::User { content, .. } => {
                ("user", Some(content.clone()))
            }
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

        Self {
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

            role: role.to_string(),
        }
    }
}
