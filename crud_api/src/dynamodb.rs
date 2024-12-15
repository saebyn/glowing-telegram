use aws_sdk_dynamodb::types::AttributeValue;
use aws_sdk_dynamodb::Client;
use lambda_runtime::Error;
use serde::Deserialize;
use serde_json::json;
use std::collections::HashMap;

#[derive(Debug, Deserialize)]
pub struct ListResult {
    pub items: Vec<serde_json::Value>,
    pub cursor: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct PageOptions {
    pub limit: i32,
    pub cursor: Option<serde_json::Value>,
}

#[tracing::instrument]
pub async fn list(
    client: &Client,
    table_name: &str,
    filters: HashMap<String, String>,
    page: PageOptions,
) -> Result<ListResult, Error> {
    let mut scan_input = client.scan().table_name(table_name);

    // Create the filter expression and attribute maps
    let mut filter_expressions = Vec::new();
    let mut expression_attribute_names = HashMap::new();
    let mut expression_attribute_values = HashMap::new();

    // Iterate over the filters and build the filter expression
    for (i, (key, value)) in filters.iter().enumerate() {
        let attribute_name = format!("#k{i}");
        let attribute_value = format!(":v{i}");

        filter_expressions
            .push(format!("{attribute_name} = {attribute_value}"));
        expression_attribute_names.insert(attribute_name, key.clone());
        expression_attribute_values
            .insert(attribute_value, AttributeValue::S(value.clone()));
    }

    // Apply the filter expression to the scan input
    if !filter_expressions.is_empty() {
        tracing::info!(
            "Applying filter expression: {0}",
            filter_expressions.join(" AND ")
        );
        scan_input = scan_input
            .filter_expression(filter_expressions.join(" AND"))
            .set_expression_attribute_names(Some(expression_attribute_names))
            .set_expression_attribute_values(Some(
                expression_attribute_values,
            ));
    }

    // Apply the limit and cursor to the scan input
    if let Some(cursor) = page.cursor {
        tracing::info!("Applying cursor: {cursor}");
        scan_input = scan_input
            .set_exclusive_start_key(Some(convert_json_to_hm(&cursor)));
    }
    if page.limit > 0 {
        tracing::info!("Applying limit: {0}", page.limit);
        scan_input = scan_input.limit(page.limit);
    }

    // Send the scan request to DynamoDB
    let scan_output = scan_input.send().await?;

    // Convert the scanned items to JSON
    let items = scan_output
        .items
        .unwrap_or_default()
        .iter()
        .map(|item| convert_hm_to_json(item.clone()))
        .collect::<Vec<serde_json::Value>>();

    // Create the response payload
    let payload = ListResult {
        items,
        cursor: scan_output
            .last_evaluated_key
            .map(|key| convert_hm_to_json(key).to_string()),
    };

    Ok(payload)
}

pub struct GetRecordResult(pub Option<serde_json::Value>);

pub async fn get(
    client: &Client,
    table_name: &str,
    key_name: &str,
    record_id: &str,
) -> Result<GetRecordResult, Error> {
    let query = client.get_item().table_name(table_name).key(
        key_name,
        aws_sdk_dynamodb::types::AttributeValue::S(record_id.to_string()),
    );

    match query.send().await {
        Ok(result) => result.item.map_or_else(
            || Ok(GetRecordResult(None)),
            |item| {
                let record = convert_hm_to_json(item);
                Ok(GetRecordResult(Some(record)))
            },
        ),
        Err(e) => Err(Error::from(e)),
    }
}

pub async fn create(
    client: &Client,
    table_name: &str,
    item: &serde_json::Value,
) -> Result<(), Error> {
    let item = convert_json_to_hm(item);

    // TODO populate the created_at field

    client
        .put_item()
        .table_name(table_name)
        .set_item(Some(item))
        .send()
        .await?;

    Ok(())
}

pub async fn update(
    client: &Client,
    table_name: &str,
    key_name: &str,
    record_id: &str,
    item: &serde_json::Value,
) -> Result<serde_json::Value, Error> {
    let item = convert_json_to_hm(item);

    // TODO populate the updated_at field

    let item_fields_to_update = item.iter().filter(|(k, _)| *k != key_name);

    let update_expression = item_fields_to_update
        .clone()
        .map(|(k, _)| format!("#{k} = :{k}"))
        .collect::<Vec<String>>()
        .join(", ");

    let update_expression = format!("SET {update_expression}");

    let expression_attribute_names = item_fields_to_update
        .clone()
        .map(|(k, _)| (format!("#{k}"), k.clone()))
        .collect::<HashMap<String, String>>();

    let expression_attribute_values = item_fields_to_update
        .clone()
        .map(|(k, v)| (format!(":{k}"), v.clone()))
        .collect::<HashMap<String, AttributeValue>>();

    tracing::debug!(
        "Update expression: {:?}, Expression attribute names: {:?}, Expression attribute values: {:?}",
        update_expression,
        expression_attribute_names,
        expression_attribute_values
    );

    let query = client
        .update_item()
        .table_name(table_name)
        .key(
            key_name,
            aws_sdk_dynamodb::types::AttributeValue::S(record_id.to_string()),
        )
        .set_update_expression(Some(update_expression))
        .set_expression_attribute_names(Some(expression_attribute_names))
        .set_expression_attribute_values(Some(expression_attribute_values))
        .return_values(aws_sdk_dynamodb::types::ReturnValue::AllNew);

    let result = query.send().await?;

    let item = result
        .attributes
        .unwrap_or_default()
        .iter()
        .map(|(k, v)| (k.clone(), convert_attribute_value_to_json(v.clone())))
        .collect::<HashMap<String, serde_json::Value>>();

    Ok(json!(item))
}

pub async fn delete(
    client: &Client,
    table_name: &str,
    key_name: &str,
    record_id: &str,
) -> Result<(), Error> {
    client
        .delete_item()
        .table_name(table_name)
        .key(
            key_name,
            aws_sdk_dynamodb::types::AttributeValue::S(record_id.to_string()),
        )
        .send()
        .await?;

    Ok(())
}

// Convert a hashmap with `AttributeValue`s to a JSON object
//
// # Arguments
//
// * `hm` - The hashmap to convert.
//
// # Returns
//
// A `serde_json::Value` representing the converted hashmap.
//
// # Example
//
// ```rust
// let hm = hashmap! {
//     "id".to_string() => AttributeValue::S("123".to_string()),
//     "name".to_string() => AttributeValue::S("John Doe".to_string()),
// };
//
// let json = convert_hm_to_json(hm);
// ```
//
// The `json` variable will contain the following JSON object:
//
// ```json
// {
//     "id": "123",
//     "name": "John Doe"
// }
// ```
fn convert_hm_to_json(
    hm: HashMap<String, AttributeValue>,
) -> serde_json::Value {
    hm.into_iter()
        .map(|(k, v)| (k, convert_attribute_value_to_json(v)))
        .collect()
}

fn convert_json_to_hm(
    json: &serde_json::Value,
) -> HashMap<String, AttributeValue> {
    json.as_object()
        .unwrap()
        .iter()
        .map(|(k, v)| (k.clone(), convert_json_to_attribute_value(v.clone())))
        .collect()
}

/// Converts a ``DynamoDB`` attribute value to a JSON value.
///
/// # Arguments
///
/// * `attribute_value` - The ``DynamoDB`` attribute value to convert.
///
/// # Returns
///
/// A `serde_json::Value` representing the converted attribute value.
fn convert_attribute_value_to_json(
    attribute_value: AttributeValue,
) -> serde_json::Value {
    match attribute_value {
        AttributeValue::S(s) => serde_json::Value::String(s),
        AttributeValue::N(n) => serde_json::Value::Number(
            serde_json::Number::from_f64(n.parse().unwrap()).unwrap(),
        ),
        AttributeValue::Bool(b) => serde_json::Value::Bool(b),
        AttributeValue::L(l) => serde_json::Value::Array(
            l.into_iter().map(convert_attribute_value_to_json).collect(),
        ),
        AttributeValue::M(m) => serde_json::Value::Object(
            m.into_iter()
                .map(|(k, v)| (k, convert_attribute_value_to_json(v)))
                .collect(),
        ),
        AttributeValue::Ss(ss) => serde_json::Value::Array(
            ss.into_iter().map(serde_json::Value::String).collect(),
        ),
        AttributeValue::Ns(ns) => serde_json::Value::Array(
            ns.into_iter()
                .map(|n| {
                    serde_json::Value::Number(
                        serde_json::Number::from_f64(n.parse().unwrap())
                            .unwrap(),
                    )
                })
                .collect(),
        ),
        _ => serde_json::Value::Null,
    }
}

fn convert_json_to_attribute_value(json: serde_json::Value) -> AttributeValue {
    match json {
        serde_json::Value::String(s) => AttributeValue::S(s),
        serde_json::Value::Number(n) => AttributeValue::N(n.to_string()),
        serde_json::Value::Bool(b) => AttributeValue::Bool(b),
        serde_json::Value::Array(a) => AttributeValue::L(
            a.into_iter().map(convert_json_to_attribute_value).collect(),
        ),
        serde_json::Value::Object(o) => AttributeValue::M(
            o.into_iter()
                .map(|(k, v)| (k, convert_json_to_attribute_value(v)))
                .collect(),
        ),
        serde_json::Value::Null => AttributeValue::Null(true),
    }
}
