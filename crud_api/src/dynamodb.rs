use aws_sdk_dynamodb::types::{AttributeValue, KeysAndAttributes};
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
    pub cursor: Option<String>,
}

const DEFAULT_PAGE_LIMIT: i32 = 10;

#[tracing::instrument(skip(client))]
pub async fn list(
    client: &Client,
    table_name: &str,
    key_name: &str,
    filters: serde_json::Map<String, serde_json::Value>,
    page: PageOptions,
) -> Result<ListResult, Error> {
    let mut items = Vec::new();
    let mut last_key = page.cursor.map(|c| {
        HashMap::from([(key_name.to_string(), AttributeValue::S(c))])
    });
    let limit = if page.limit > 0 {
        page.limit
    } else {
        DEFAULT_PAGE_LIMIT
    };

    let (
        filter_expressions,
        expression_attribute_names,
        expression_attribute_values,
    ) = build_filter_expressions(&filters);

    loop {
        tracing::info!(
            "Scanning table: {0}, with limit: {1}, cursor: {2:?}",
            table_name,
            limit,
            last_key
        );
        let mut scan_input = client.scan().table_name(table_name);

        // Apply the filter expression to the scan input
        if !filter_expressions.is_empty() {
            tracing::info!(
                "Applying filter expression: {0}",
                filter_expressions.join(" AND ")
            );
            scan_input = scan_input
                .filter_expression(filter_expressions.join(" AND "))
                .set_expression_attribute_names(Some(
                    expression_attribute_names.clone(),
                ))
                .set_expression_attribute_values(Some(
                    expression_attribute_values.clone(),
                ));
        }

        // Apply the limit and cursor to the scan input
        if let Some(key) = last_key.clone() {
            scan_input = scan_input.set_exclusive_start_key(Some(key));
        }
        let remaining = limit - i32::try_from(items.len())?;
        if remaining <= 0 {
            tracing::info!("Reached the limit of {0} items", limit);
            break;
        }
        scan_input = scan_input.limit(remaining);

        // Send the scan request to DynamoDB
        let scan_output = scan_input.send().await?;

        // Convert the scanned items to JSON
        let new_items = scan_output
            .items
            .unwrap_or_default()
            .iter()
            .map(|item| convert_hm_to_json(item.clone()))
            .collect::<Vec<serde_json::Value>>();
        items.extend(new_items);

        if let Some(k) = scan_output.last_evaluated_key {
            last_key = Some(k);
        } else {
            // No more items to scan
            tracing::info!("No more items to scan");
            last_key = None;
            break;
        }
    }

    tracing::info!("Returning {0} items", items.len());

    // Create the response payload
    let payload = ListResult {
        items,
        cursor: last_key
            .map(|k| k.get(key_name).unwrap().as_s().unwrap().to_string()),
    };

    Ok(payload)
}

pub struct GetRecordResult(pub Option<serde_json::Value>);

#[tracing::instrument(skip(client))]
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

#[tracing::instrument(skip(client))]
pub async fn get_many(
    client: &Client,
    table_name: &str,
    key_name: &str,
    ids: &[&str],
) -> Result<Vec<serde_json::Value>, Error> {
    let keys = ids
        .iter()
        .map(|id| {
            vec![(key_name.to_string(), AttributeValue::S((*id).to_string()))]
                .into_iter()
                .collect()
        })
        .collect();

    let request_items = std::collections::HashMap::from([(
        table_name.to_string(),
        KeysAndAttributes::builder().set_keys(Some(keys)).build()?,
    )]);

    let resp = client
        .batch_get_item()
        .set_request_items(Some(request_items))
        .send()
        .await?;

    let mut items = Vec::new();
    if let Some(responses) = resp.responses() {
        if let Some(table_items) = responses.get(table_name) {
            for item in table_items {
                items.push(convert_hm_to_json(item.clone()));
            }
        }
    }
    Ok(items)
}

#[tracing::instrument(skip(client))]
pub async fn create(
    client: &Client,
    table_name: &str,
    item: &serde_json::Value,
) -> Result<(), Error> {
    let mut item = convert_json_to_hm(item);

    // populate the created_at field
    let created_at = chrono::Utc::now().to_rfc3339();
    item.insert(
        "created_at".to_string(),
        AttributeValue::S(created_at.to_string()),
    );

    client
        .put_item()
        .table_name(table_name)
        .set_item(Some(item))
        .send()
        .await?;

    Ok(())
}

#[tracing::instrument(skip(client))]
pub async fn update(
    client: &Client,
    table_name: &str,
    key_name: &str,
    record_id: &str,
    item: &serde_json::Value,
) -> Result<serde_json::Value, Error> {
    let mut item = convert_json_to_hm(item);

    // populate the updated_at field with the current timestamp,
    // replacing the existing value if it exists
    let updated_at = chrono::Utc::now().to_rfc3339();
    item.insert(
        "updated_at".to_string(),
        AttributeValue::S(updated_at.to_string()),
    );

    let (
        update_expression,
        expression_attribute_names,
        expression_attribute_values,
    ) = item
        .iter()
        .filter(|(k, _)| *k != key_name)
        .enumerate()
        .fold(
            (Vec::new(), HashMap::new(), HashMap::new()),
            |(mut exprs, mut names, mut values), (i, (k, v))| {
                exprs.push(format!("#k{i} = :v{i}"));
                names.insert(format!("#k{i}"), k.clone());
                values.insert(format!(":v{i}"), v.clone());
                (exprs, names, values)
            },
        );
    let update_expression = format!("SET {}", update_expression.join(", "));

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

#[tracing::instrument(skip(client))]
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

fn get_operator(op_name: &str) -> &'static str {
    match op_name {
        "gte" => ">=",
        "gt" => ">",
        "lte" => "<=",
        "lt" => "<",
        _ => "=",
    }
}

fn build_filter_expressions(
    filters: &serde_json::Map<String, serde_json::Value>,
) -> (
    Vec<String>,
    HashMap<String, String>,
    HashMap<String, AttributeValue>,
) {
    // Create the filter expression and attribute maps
    let mut filter_expressions = Vec::new();
    let mut expression_attribute_names = HashMap::new();
    let mut expression_attribute_values = HashMap::new();

    // Iterate over the filters and build the filter expression
    for (i, (key, value)) in filters.iter().enumerate() {
        let (base_key, op) =
            if let Some((name, suffix)) = key.rsplit_once("__") {
                (name, get_operator(suffix))
            } else {
                (key.as_str(), "=")
            };
        let attribute_name = format!("#k{i}");
        let attribute_value = format!(":v{i}");

        filter_expressions
            .push(format!("{attribute_name} {op} {attribute_value}"));
        expression_attribute_names
            .insert(attribute_name.clone(), base_key.to_string());
        expression_attribute_values.insert(
            attribute_value,
            convert_json_to_attribute_value(value.clone()),
        );
    }

    (
        filter_expressions,
        expression_attribute_names,
        expression_attribute_values,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_filter_expressions_equality() {
        let mut filters = serde_json::Map::new();
        filters.insert(
            "status".to_string(),
            serde_json::Value::String("active".to_string()),
        );
        let (exprs, anames, avals) = build_filter_expressions(&filters);
        assert_eq!(exprs, vec!["#k0 = :v0"]);
        assert_eq!(anames.get("#k0"), Some(&"status".to_string()));
        assert_eq!(
            avals.get(":v0").unwrap().as_s().ok(),
            Some("active".to_string()).as_ref()
        );
    }

    #[test]
    fn test_build_filter_expressions_greater_than() {
        let mut filters = serde_json::Map::new();
        filters.insert(
            "age__gt".to_string(),
            serde_json::Value::Number(30.into()),
        );
        let (exprs, anames, avals) = build_filter_expressions(&filters);
        assert_eq!(exprs, vec!["#k0 > :v0"]);
        assert_eq!(anames.get("#k0"), Some(&"age".to_string()));
        assert_eq!(
            avals.get(":v0").unwrap().as_n().ok(),
            Some("30".to_string()).as_ref()
        );
    }
}
