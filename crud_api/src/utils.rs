use aws_sdk_dynamodb::types::AttributeValue;
use serde_json::Value;
use std::collections::HashMap;

use crate::dynamodb::DynamoDbTableConfig;

pub fn deserialize_cursor(cursor: &str) -> HashMap<String, AttributeValue> {
    let cursor: HashMap<String, String> =
        serde_json::from_str(cursor).unwrap_or_default();

    cursor
        .into_iter()
        .map(|(k, v)| (k, AttributeValue::S(v)))
        .collect()
}

pub fn serialize_cursor(cursor: &HashMap<String, AttributeValue>) -> String {
    let cursor: HashMap<String, String> = cursor
        .iter()
        .filter_map(|(k, v)| match v {
            AttributeValue::S(s) => Some((k.clone(), s.clone())),
            _ => None,
        })
        .collect();

    serde_json::to_string(&cursor).unwrap_or_default()
}

pub fn extract_id_from_item(
    table_config: &DynamoDbTableConfig,
    item: &Value,
) -> Value {
    // if table has a partition key but no sort key, use the partition key as the id
    if table_config.sort_key.is_none() {
        return item
            .get(table_config.partition_key)
            .and_then(|v| {
                v.as_str().map(|s| serde_json::Value::String(s.to_string()))
            })
            .unwrap_or(serde_json::Value::Null);
    }

    // otherwise, serialize the partition and sort keys as a JSON string
    let partition_key = item
        .get(table_config.partition_key)
        .and_then(|v| v.as_str().map(|s| s.to_string()))
        .unwrap_or_default();
    let sort_key = item
        .get(table_config.sort_key.unwrap())
        .and_then(|v| v.as_str().map(|s| s.to_string()))
        .unwrap_or_default();

    serde_json::Value::String(
        serde_json::to_string(&vec![partition_key, sort_key])
            .unwrap_or_default(),
    )
}
