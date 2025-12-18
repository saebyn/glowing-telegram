use aws_lambda_events::event::sqs::{SqsEvent, SqsMessage};
use aws_sdk_dynamodb::Client as DynamoDbClient;
use aws_sdk_dynamodb::types::AttributeValue;
use chrono::Utc;
use lambda_runtime::{Error, LambdaEvent, service_fn};
use serde::Deserialize;
use std::collections::HashMap;
use tracing::{error, info};

#[derive(Debug, Deserialize, Clone)]
#[allow(clippy::struct_field_names)]
struct Config {
    chat_messages_table: String,
    chat_message_ttl_days: i64,
}

#[derive(Debug, Clone)]
struct AppContext {
    dynamodb: DynamoDbClient,
    config: Config,
}

impl gt_app::ContextProvider<Config> for AppContext {
    async fn new(config: Config, aws_config: aws_config::SdkConfig) -> Self {
        Self {
            config,
            dynamodb: DynamoDbClient::new(&aws_config),
        }
    }
}

#[derive(Debug, Deserialize)]
struct TwitchChatEvent {
    broadcaster_user_id: String,
    broadcaster_user_name: String,
    broadcaster_user_login: String,
    chatter_user_id: String,
    chatter_user_name: String,
    chatter_user_login: String,
    message_id: String,
    message: TwitchChatMessage,
    color: Option<String>,
    message_type: Option<String>,
}

#[derive(Debug, Deserialize)]
struct TwitchChatMessage {
    text: String,
}

#[derive(Debug, Deserialize)]
struct EventSubMessage {
    subscription: EventSubSubscription,
    event: TwitchChatEvent,
}

#[derive(Debug, Deserialize)]
struct EventSubSubscription {
    #[serde(rename = "type")]
    event_type: String,
    id: String,
}

#[tokio::main]
async fn main() -> Result<(), Error> {
    let context = gt_app::create_app_context::<AppContext, Config>().await?;

    lambda_runtime::run(service_fn(|event| {
        let context = context.clone();
        async move { handler(event, context).await }
    }))
    .await
}

async fn handler(
    event: LambdaEvent<SqsEvent>,
    context: AppContext,
) -> Result<(), Error> {
    info!("Processing {} SQS messages", event.payload.records.len());

    for record in event.payload.records {
        if let Err(e) = process_message(&record, &context).await {
            error!("Failed to process message: {:?}", e);
            // Continue processing other messages even if one fails
        }
    }

    Ok(())
}

async fn process_message(
    message: &SqsMessage,
    context: &AppContext,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let body = message.body.as_ref().ok_or("No message body")?;

    info!("Processing message: {}", body);

    // Parse the EventSub message
    let eventsub_message: EventSubMessage = serde_json::from_str(body)?;

    if eventsub_message.subscription.event_type != "channel.chat.message" {
        info!(
            "Ignoring non-chat message event: {}",
            eventsub_message.subscription.event_type
        );
        return Ok(());
    }

    let event = eventsub_message.event;
    let chatter_name = event.chatter_user_name.clone();

    // Create a chat message record for DynamoDB
    let mut item = HashMap::new();

    // Use the broadcaster (channel owner) as the user_id for partitioning
    item.insert(
        "user_id".to_string(),
        AttributeValue::S(event.broadcaster_user_id.clone()),
    );

    item.insert(
        "timestamp".to_string(),
        AttributeValue::S(Utc::now().to_rfc3339()),
    );

    item.insert(
        "sender_id".to_string(),
        AttributeValue::S(event.chatter_user_id),
    );

    item.insert(
        "channel_id".to_string(),
        AttributeValue::S(event.broadcaster_user_id),
    );

    item.insert("message".to_string(), AttributeValue::S(event.message.text));

    item.insert(
        "user_name".to_string(),
        AttributeValue::S(event.chatter_user_name),
    );

    item.insert(
        "user_login".to_string(),
        AttributeValue::S(event.chatter_user_login),
    );

    item.insert(
        "event_type".to_string(),
        AttributeValue::S("channel.chat.message".to_string()),
    );

    // Set TTL to some number of days from now
    let ttl = Utc::now().timestamp()
        + (context.config.chat_message_ttl_days * 24 * 60 * 60);
    item.insert("ttl".to_string(), AttributeValue::N(ttl.to_string()));

    // Store in DynamoDB
    context
        .dynamodb
        .put_item()
        .table_name(&context.config.chat_messages_table)
        .set_item(Some(item))
        .send()
        .await?;

    info!("Successfully stored chat message from {}", chatter_name);

    Ok(())
}
