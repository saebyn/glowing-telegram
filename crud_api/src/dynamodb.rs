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

#[derive(Debug)]
pub struct DynamoDbTableConfig<'a> {
    pub table: &'a str,
    pub partition_key: &'a str,
    pub q_key: &'a str,
    pub indexes: Vec<&'a str>,
}

const DEFAULT_PAGE_LIMIT: i32 = 10;

#[tracing::instrument(skip(client))]
pub async fn list(
    client: &Client,
    table_config: &DynamoDbTableConfig<'_>,
    filters: serde_json::Map<String, serde_json::Value>,
    page: PageOptions,
) -> Result<ListResult, Error> {
    let mut items = Vec::new();
    let mut last_key = page.cursor.map(|c| {
        HashMap::from([(
            table_config.partition_key.to_string(),
            AttributeValue::S(c),
        )])
    });
    let limit = if page.limit > 0 {
        page.limit
    } else {
        DEFAULT_PAGE_LIMIT
    };

    let (
        filter_expression,
        expression_attribute_names,
        expression_attribute_values,
    ) = build_filter_expressions(table_config, &filters);

    loop {
        tracing::info!(
            "Scanning table: {0}, with limit: {1}, cursor: {2:?}",
            table_config.table,
            limit,
            last_key
        );
        let mut scan_input = client.scan().table_name(table_config.table);

        // Apply the filter expression to the scan input
        if let Some(filter_expression) = filter_expression.clone() {
            tracing::info!(
                "Applying filter expression: {0}",
                filter_expression
            );
            scan_input = scan_input
                .filter_expression(filter_expression)
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
        cursor: last_key.map(|k| {
            k.get(table_config.partition_key)
                .unwrap()
                .as_s()
                .unwrap()
                .to_string()
        }),
    };

    Ok(payload)
}

#[tracing::instrument(skip(client))]
pub async fn query(
    client: &Client,
    table_config: &DynamoDbTableConfig<'_>,
    indexed_field: &str,
    value: serde_json::Value,
) -> Result<ListResult, Error> {
    let query = client
        .query()
        .table_name(table_config.table)
        .index_name(format!("{indexed_field}-index"))
        .expression_attribute_names("#k", indexed_field)
        .expression_attribute_values(
            ":v",
            convert_json_to_attribute_value(value),
        )
        .key_condition_expression("#k = :v");

    let query_output = match query.send().await {
        Ok(query_output) => query_output,
        Err(err) => {
            tracing::error!("Failed to query: {:?}", err);

            return Err(Box::new(err));
        }
    };

    let items = query_output
        .items
        .unwrap_or_default()
        .iter()
        .map(|item| convert_hm_to_json(item.clone()))
        .collect::<Vec<serde_json::Value>>();

    Ok(ListResult {
        cursor: None,
        items,
    })
}

pub struct GetRecordResult(pub Option<serde_json::Value>);

#[tracing::instrument(skip(client))]
pub async fn get(
    client: &Client,
    table_config: &DynamoDbTableConfig<'_>,
    record_id: &str,
) -> Result<GetRecordResult, Error> {
    let query = client.get_item().table_name(table_config.table).key(
        table_config.partition_key,
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
    table_config: &DynamoDbTableConfig<'_>,
    ids: &[&str],
) -> Result<Vec<serde_json::Value>, Error> {
    let keys = ids
        .iter()
        .map(|id| {
            vec![(
                table_config.partition_key.to_string(),
                AttributeValue::S((*id).to_string()),
            )]
            .into_iter()
            .collect()
        })
        .collect();

    let request_items = std::collections::HashMap::from([(
        table_config.table.to_string(),
        KeysAndAttributes::builder().set_keys(Some(keys)).build()?,
    )]);

    let resp = client
        .batch_get_item()
        .set_request_items(Some(request_items))
        .send()
        .await?;

    let mut items = Vec::new();
    if let Some(responses) = resp.responses() {
        if let Some(table_items) = responses.get(table_config.table) {
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
    table_config: &DynamoDbTableConfig<'_>,
    item: &serde_json::Value,
) -> Result<(), Error> {
    let mut item = convert_json_to_hm(item);

    // TODO generate a UUID for the record ID, handle if the ID is already present

    // populate the created_at field
    let created_at = chrono::Utc::now().to_rfc3339();
    item.insert(
        "created_at".to_string(),
        AttributeValue::S(created_at.to_string()),
    );

    client
        .put_item()
        .table_name(table_config.table)
        .set_item(Some(item))
        .send()
        .await?;

    Ok(())
}

#[tracing::instrument(skip(client))]
pub async fn update(
    client: &Client,
    table_config: &DynamoDbTableConfig<'_>,
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
        .filter(|(k, _)| *k != table_config.partition_key)
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
        .table_name(table_config.table)
        .key(
            table_config.partition_key,
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
    table_config: &DynamoDbTableConfig<'_>,
    record_id: &str,
) -> Result<(), Error> {
    client
        .delete_item()
        .table_name(table_config.table)
        .key(
            table_config.partition_key,
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
    table_config: &DynamoDbTableConfig<'_>,
    filters: &serde_json::Map<String, serde_json::Value>,
) -> (
    Option<String>,
    HashMap<String, String>,
    HashMap<String, AttributeValue>,
) {
    // Create the filter expression and attribute maps
    let mut filter_expressions = Vec::new();
    let mut expression_attribute_names = HashMap::new();
    let mut expression_attribute_values = HashMap::new();

    // Iterate over the filters and build the filter expression
    for (i, (key, value)) in filters.iter().enumerate() {
        // Check if the key contains an operator suffix (e.g. __gt)
        let (base_key, op) =
            if let Some((name, suffix)) = key.rsplit_once("__") {
                (name, get_operator(suffix))
            } else {
                (key.as_str(), "=")
            };

        // Handle the "q" key name separately, converting it to the actual key name for the table from the config
        let (base_key, op) = if base_key == "q" {
            (table_config.q_key, "contains")
        } else {
            (base_key, op)
        };

        let attribute_name = format!("#k{i}");
        let attribute_value = format!(":v{i}");

        filter_expressions.push(if op == "contains" {
            format!("contains({attribute_name}, {attribute_value})")
        } else {
            format!("{attribute_name} {op} {attribute_value}")
        });

        expression_attribute_names
            .insert(attribute_name.clone(), base_key.to_string());
        expression_attribute_values.insert(
            attribute_value,
            convert_json_to_attribute_value(value.clone()),
        );
    }

    (
        if filter_expressions.is_empty() {
            None
        } else {
            Some(filter_expressions.join(" AND "))
        },
        expression_attribute_names,
        expression_attribute_values,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_filter_expressions_equality() {
        let table_config = DynamoDbTableConfig {
            table: "users",
            partition_key: "id",
            q_key: "name",
            indexes: vec![],
        };
        let mut filters = serde_json::Map::new();
        filters.insert(
            "status".to_string(),
            serde_json::Value::String("active".to_string()),
        );
        let (expr, anames, avals) =
            build_filter_expressions(&table_config, &filters);
        assert_eq!(expr.unwrap(), "#k0 = :v0");
        assert_eq!(anames.get("#k0"), Some(&"status".to_string()));
        assert_eq!(
            avals.get(":v0").unwrap().as_s().ok(),
            Some("active".to_string()).as_ref()
        );
    }

    #[test]
    fn test_build_filter_expressions_greater_than() {
        let table_config = DynamoDbTableConfig {
            table: "users",
            partition_key: "id",
            q_key: "name",
            indexes: vec![],
        };
        let mut filters = serde_json::Map::new();
        filters.insert(
            "age__gt".to_string(),
            serde_json::Value::Number(30.into()),
        );
        let (expr, anames, avals) =
            build_filter_expressions(&table_config, &filters);
        assert_eq!(expr.unwrap(), "#k0 > :v0");
        assert_eq!(anames.get("#k0"), Some(&"age".to_string()));
        assert_eq!(
            avals.get(":v0").unwrap().as_n().ok(),
            Some("30".to_string()).as_ref()
        );
    }
}
