use aws_sdk_dynamodb::{
    Client as DynamoDbClient,
    types::{AttributeValue, WriteRequest},
};
use chrono::Utc;
use gt_app::ContextProvider;
use lambda_runtime::{Error, LambdaEvent, run, service_fn};
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use std::collections::HashMap;
use tracing::{info, warn};

mod updaters;
use updaters::{WidgetUpdate, WidgetUpdater, countdown::CountdownUpdater};

#[derive(Debug, Deserialize)]
struct ScheduledEvent {
    widget_type: String,
}

#[derive(Debug, Serialize)]
struct Response {
    widgets_processed: usize,
    widgets_updated: usize,
}

#[derive(Debug, Clone, Deserialize)]
struct Config {
    stream_widgets_table: String,
}

#[derive(Debug, Clone)]
struct AppContext {
    dynamodb: DynamoDbClient,
    config: Config,
}

impl ContextProvider<Config> for AppContext {
    async fn new(config: Config, aws_config: aws_config::SdkConfig) -> Self {
        Self {
            config,
            dynamodb: DynamoDbClient::new(&aws_config),
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
struct StreamWidget {
    id: String,
    #[serde(rename = "type")]
    widget_type: String,
    active: Option<bool>,
    config: Option<HashMap<String, JsonValue>>,
    state: Option<HashMap<String, JsonValue>>,
}

async fn function_handler(
    context: &AppContext,
    event: LambdaEvent<ScheduledEvent>,
) -> Result<Response, Error> {
    let widget_type = &event.payload.widget_type;
    info!("Processing {} widgets", widget_type);

    // Query all active widgets of this type
    let active_widgets =
        query_active_widgets_by_type(context, widget_type).await?;

    if active_widgets.is_empty() {
        info!("No active {} widgets to update", widget_type);
        return Ok(Response {
            widgets_processed: 0,
            widgets_updated: 0,
        });
    }

    info!(
        "Found {} active {} widgets",
        active_widgets.len(),
        widget_type
    );

    // Get the appropriate updater for this widget type
    let updater = get_updater_for_type(widget_type)?;

    // Compute updates for all widgets
    let updates = updater.compute_batch_updates(&active_widgets);

    info!("Generated {} updates", updates.len());

    // Write updates to DynamoDB in batches
    let updated_count = batch_write_widget_states(context, &updates).await?;

    Ok(Response {
        widgets_processed: active_widgets.len(),
        widgets_updated: updated_count,
    })
}

async fn query_active_widgets_by_type(
    context: &AppContext,
    widget_type: &str,
) -> Result<Vec<StreamWidget>, Error> {
    let mut widgets = Vec::new();
    let mut exclusive_start_key: Option<HashMap<String, AttributeValue>> =
        None;

    loop {
        let mut query = context
            .dynamodb
            .query()
            .table_name(&context.config.stream_widgets_table)
            .index_name("type-active-index")
            .key_condition_expression(
                "#type = :widget_type AND active = :active",
            )
            .expression_attribute_names("#type", "type")
            .expression_attribute_values(
                ":widget_type",
                AttributeValue::S(widget_type.to_string()),
            )
            .expression_attribute_values(
                ":active",
                AttributeValue::N("1".to_string()),
            );

        if let Some(start_key) = exclusive_start_key {
            query = query.set_exclusive_start_key(Some(start_key));
        }

        let response = query.send().await?;

        if let Some(items) = response.items {
            for item in items {
                if let Ok(widget) = deserialize_widget(&item) {
                    widgets.push(widget);
                } else {
                    warn!("Failed to deserialize widget: {:?}", item);
                }
            }
        }

        if response.last_evaluated_key.is_none() {
            break;
        }
        exclusive_start_key = response.last_evaluated_key;
    }

    Ok(widgets)
}

fn deserialize_widget(
    item: &HashMap<String, AttributeValue>,
) -> Result<StreamWidget, Error> {
    let id = item
        .get("id")
        .and_then(|v| v.as_s().ok())
        .ok_or("Missing id")?
        .clone();

    let widget_type = item
        .get("type")
        .and_then(|v| v.as_s().ok())
        .ok_or("Missing type")?
        .clone();

    let active = item
        .get("active")
        .and_then(|v| v.as_n().ok())
        .and_then(|n| n.parse::<u8>().ok())
        .map(|n| n == 1);

    let config = item.get("config").and_then(|v| deserialize_map(v).ok());

    let state = item.get("state").and_then(|v| deserialize_map(v).ok());

    Ok(StreamWidget {
        id,
        widget_type,
        active,
        config,
        state,
    })
}

fn deserialize_map(
    attr: &AttributeValue,
) -> Result<HashMap<String, JsonValue>, Error> {
    match attr {
        AttributeValue::M(map) => {
            let mut result = HashMap::new();
            for (key, value) in map {
                result.insert(key.clone(), attribute_value_to_json(value)?);
            }
            Ok(result)
        }
        _ => Err("Expected map".into()),
    }
}

fn attribute_value_to_json(attr: &AttributeValue) -> Result<JsonValue, Error> {
    match attr {
        AttributeValue::S(s) => Ok(JsonValue::String(s.clone())),
        AttributeValue::N(n) => {
            if let Ok(i) = n.parse::<i64>() {
                Ok(JsonValue::Number(i.into()))
            } else if let Ok(f) = n.parse::<f64>() {
                Ok(serde_json::Number::from_f64(f)
                    .map(JsonValue::Number)
                    .unwrap_or(JsonValue::Null))
            } else {
                Ok(JsonValue::Null)
            }
        }
        AttributeValue::Bool(b) => Ok(JsonValue::Bool(*b)),
        AttributeValue::M(map) => {
            let mut result = serde_json::Map::new();
            for (key, value) in map {
                result.insert(key.clone(), attribute_value_to_json(value)?);
            }
            Ok(JsonValue::Object(result))
        }
        AttributeValue::L(list) => {
            let mut result = Vec::new();
            for value in list {
                result.push(attribute_value_to_json(value)?);
            }
            Ok(JsonValue::Array(result))
        }
        AttributeValue::Null(_) => Ok(JsonValue::Null),
        _ => Ok(JsonValue::Null),
    }
}

async fn batch_write_widget_states(
    context: &AppContext,
    updates: &[WidgetUpdate],
) -> Result<usize, Error> {
    let now = Utc::now().to_rfc3339();
    let mut success_count = 0;

    // Process updates in batches of 25 (DynamoDB limit)
    for chunk in updates.chunks(25) {
        let mut write_requests = Vec::new();

        for update in chunk {
            let mut item = HashMap::new();
            item.insert(
                "id".to_string(),
                AttributeValue::S(update.id.clone()),
            );
            item.insert(
                "state".to_string(),
                json_to_attribute_value(&serde_json::to_value(&update.state)?),
            );
            item.insert(
                "updated_at".to_string(),
                AttributeValue::S(now.clone()),
            );

            write_requests.push(
                WriteRequest::builder()
                    .put_request(
                        aws_sdk_dynamodb::types::PutRequest::builder()
                            .set_item(Some(item))
                            .build()
                            .map_err(|e| {
                                format!("Failed to build put request: {}", e)
                            })?,
                    )
                    .build(),
            );
        }

        match context
            .dynamodb
            .batch_write_item()
            .request_items(
                &context.config.stream_widgets_table,
                write_requests,
            )
            .send()
            .await
        {
            Ok(_) => {
                success_count += chunk.len();
            }
            Err(e) => {
                warn!("Failed to write batch: {}", e);
                // Continue processing remaining batches even if one fails
            }
        }
    }

    Ok(success_count)
}

fn json_to_attribute_value(value: &JsonValue) -> AttributeValue {
    match value {
        JsonValue::String(s) => AttributeValue::S(s.clone()),
        JsonValue::Number(n) => AttributeValue::N(n.to_string()),
        JsonValue::Bool(b) => AttributeValue::Bool(*b),
        JsonValue::Object(map) => {
            let mut attr_map = HashMap::new();
            for (key, val) in map {
                attr_map.insert(key.clone(), json_to_attribute_value(val));
            }
            AttributeValue::M(attr_map)
        }
        JsonValue::Array(arr) => {
            let attr_list: Vec<AttributeValue> =
                arr.iter().map(json_to_attribute_value).collect();
            AttributeValue::L(attr_list)
        }
        JsonValue::Null => AttributeValue::Null(true),
    }
}

fn get_updater_for_type(
    widget_type: &str,
) -> Result<Box<dyn WidgetUpdater>, Error> {
    match widget_type {
        "countdown" => Ok(Box::new(CountdownUpdater)),
        _ => Err(format!("Unknown widget type: {}", widget_type).into()),
    }
}

#[tokio::main]
async fn main() -> Result<(), Error> {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .with_target(false)
        .without_time()
        .init();

    let context = gt_app::create_app_context::<AppContext, Config>().await?;

    run(service_fn(|event| async {
        function_handler(&context, event).await
    }))
    .await
}
