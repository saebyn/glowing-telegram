use aws_sdk_dynamodb::types::AttributeValue;
use aws_sdk_dynamodb::Client;
use lambda_runtime::Error;
use serde::Deserialize;
use std::collections::HashMap;

#[derive(Debug, Deserialize)]
pub struct ListResult {
    pub items: Vec<serde_json::Value>,
    pub cursor: Option<String>,
    pub total_items: i32,
}

#[derive(Debug, Deserialize)]
pub struct PageOptions {
    pub limit: i32,
    pub cursor: Option<String>,
}

pub async fn list(
    client: &Client,
    table_name: &str,
    sort: Option<String>,
    filters: HashMap<String, String>,
    page: PageOptions,
) -> Result<ListResult, Error> {
    let mut scan_input = client.scan().table_name(table_name);

    // Check if the query contains a filter and parse it

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
        scan_input = scan_input
            .filter_expression(filter_expressions.join(" AND"))
            .set_expression_attribute_names(Some(expression_attribute_names))
            .set_expression_attribute_values(Some(
                expression_attribute_values,
            ));
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
        total_items: scan_output.count,
    };

    Ok(payload)
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
