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
    stream_widgets_table: String,
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

/// Payload for channel.ad_break.begin EventSub events.
#[derive(Debug, Deserialize)]
struct TwitchAdBreakEvent {
    /// Twitch broadcaster user ID (not Cognito user ID).
    broadcaster_user_id: String,
    /// Duration of the ad break in seconds.
    duration_seconds: i64,
    /// ISO 8601 timestamp when the ad break started.
    started_at: String,
}

#[derive(Debug, Deserialize)]
struct EventSubMessage {
    subscription: EventSubSubscription,
    event: serde_json::Value,
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

    match eventsub_message.subscription.event_type.as_str() {
        "channel.chat.message" => {
            let event: TwitchChatEvent =
                serde_json::from_value(eventsub_message.event)?;
            process_chat_message(event, context).await
        }
        "channel.ad_break.begin" => {
            let event: TwitchAdBreakEvent =
                serde_json::from_value(eventsub_message.event)?;
            process_ad_break_begin(event, context).await
        }
        other => {
            info!("Ignoring unhandled EventSub event type: {}", other);
            Ok(())
        }
    }
}

async fn process_chat_message(
    event: TwitchChatEvent,
    context: &AppContext,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
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

/// Handle a channel.ad_break.begin EventSub event.
///
/// Finds all active ad_timer widgets for this broadcaster and sets their state
/// to in_ad_break so OBS sees the update immediately via DynamoDB Streams →
/// WebSocket push, without waiting for the next polling cycle.
async fn process_ad_break_begin(
    event: TwitchAdBreakEvent,
    context: &AppContext,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    info!(
        "Ad break began for broadcaster {} — {} seconds, started at {}",
        event.broadcaster_user_id, event.duration_seconds, event.started_at
    );

    // Compute when the ad break ends so the frontend can derive status correctly.
    let back_from_ads_until =
        chrono::DateTime::parse_from_rfc3339(&event.started_at)
            .ok()
            .map(|t| {
                (t + chrono::Duration::seconds(event.duration_seconds))
                    .to_rfc3339()
            });

    // Query the user_id-index GSI to efficiently find active ad_timer widgets
    // for this broadcaster, and paginate through all results.
    let mut items: Vec<HashMap<String, AttributeValue>> = Vec::new();
    let mut exclusive_start_key: Option<HashMap<String, AttributeValue>> = None;

    loop {
        let mut query_builder = context
            .dynamodb
            .query()
            .table_name(&context.config.stream_widgets_table)
            .index_name("user_id-index")
            .key_condition_expression("#user_id = :broadcaster_id")
            .filter_expression("#type = :widget_type AND #active = :active")
            .expression_attribute_names("#user_id", "user_id")
            .expression_attribute_names("#type", "type")
            .expression_attribute_names("#active", "active")
            .expression_attribute_values(
                ":broadcaster_id",
                AttributeValue::S(event.broadcaster_user_id.clone()),
            )
            .expression_attribute_values(
                ":widget_type",
                AttributeValue::S("ad_timer".to_string()),
            )
            .expression_attribute_values(
                ":active",
                AttributeValue::Bool(true),
            );

        if let Some(start_key) = exclusive_start_key.take() {
            query_builder =
                query_builder.set_exclusive_start_key(Some(start_key));
        }

        let query_result = query_builder.send().await?;

        if let Some(mut page_items) = query_result.items {
            items.append(&mut page_items);
        }

        match query_result.last_evaluated_key {
            Some(key) if !key.is_empty() => {
                exclusive_start_key = Some(key);
            }
            _ => break,
        }
    }
    info!(
        "Found {} active ad_timer widgets for broadcaster {}",
        items.len(),
        event.broadcaster_user_id
    );

    let now = Utc::now().to_rfc3339();

    for item in &items {
        let Some(AttributeValue::S(widget_id)) = item.get("id") else {
            continue;
        };

        // Update only the relevant nested fields within the existing state map.
        // nextAdAt is cleared since we are now in the break.
        // backFromAdsUntil marks when the break ends.
        // This preserves any other state fields (e.g. snoozeCount, snoozedAt).
        let base_update = context
            .dynamodb
            .update_item()
            .table_name(&context.config.stream_widgets_table)
            .key("id", AttributeValue::S(widget_id.clone()))
            .expression_attribute_names("#state", "state")
            .expression_attribute_names("#nextAdAt", "nextAdAt")
            .expression_attribute_names("#backFromAdsUntil", "backFromAdsUntil")
            .expression_attribute_names("#updated_at", "updated_at")
            .expression_attribute_values(
                ":next_ad_at",
                AttributeValue::Null(true),
            )
            .expression_attribute_values(
                ":updated_at",
                AttributeValue::S(now.clone()),
            );

        let update = if let Some(ref until) = back_from_ads_until {
            base_update
                .update_expression(
                    "SET #state.#nextAdAt = :next_ad_at, \
                     #state.#backFromAdsUntil = :back_from_ads_until, \
                     #updated_at = :updated_at",
                )
                .expression_attribute_values(
                    ":back_from_ads_until",
                    AttributeValue::S(until.clone()),
                )
        } else {
            base_update.update_expression(
                "SET #state.#nextAdAt = :next_ad_at, \
                 #updated_at = :updated_at \
                 REMOVE #state.#backFromAdsUntil",
            )
        };

        let result = update.send().await;

        match result {
            Ok(_) => {
                info!("Updated ad_timer widget {} to in_ad_break", widget_id)
            }
            Err(e) => error!("Failed to update widget {}: {:?}", widget_id, e),
        }
    }

    Ok(())
}
