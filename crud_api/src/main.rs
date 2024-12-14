/**
 * This is the main entrypoint for the `crud_api` lambda function.
 *
 * The function is responsible for handling the requests and responses for the
 * CRUD operations, in a way compatible with the ra-data-simple-rest data
 * provider for React Admin.
 *
 */
use aws_config::{meta::region::RegionProviderChain, BehaviorVersion};
use aws_sdk_dynamodb::Client;
use figment::Figment;
use lambda_runtime::{service_fn, Error, LambdaEvent};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::collections::HashMap;

mod dynamodb;

#[derive(Debug, Deserialize)]
#[allow(clippy::struct_field_names)]
struct Config {
    video_metadata_table: String,
    episodes_table: String,
    streams_table: String,
    series_table: String,
}

fn load_config() -> Result<Config, figment::Error> {
    let figment = Figment::new().merge(figment::providers::Env::raw());

    figment.extract()
}

struct SharedResources {
    dynamodb: Client,
    config: Config,
}

#[derive(Debug, Deserialize)]
struct Request {
    resource: String,
    method: String,
    record_id: Option<String>,
    query: HashMap<String, String>,
    payload: Option<String>,
}

#[derive(Serialize)]
struct Response {
    payload: serde_json::Value,
    headers: HashMap<String, String>,
    status_code: u16,
}

#[tokio::main]
async fn main() -> Result<(), Error> {
    let config = load_config().expect("failed to load config");
    let region_provider =
        RegionProviderChain::default_provider().or_else("us-east-1");
    let aws_config = aws_config::defaults(BehaviorVersion::latest())
        .region(region_provider)
        .load()
        .await;

    let dynamodb = aws_sdk_dynamodb::Client::new(&aws_config);

    let shared_resources = &SharedResources { dynamodb, config };

    let func = service_fn(move |event: LambdaEvent<Request>| async move {
        handler(shared_resources, event).await
    });

    lambda_runtime::run(func).await?;

    Ok(())
}

async fn handler(
    shared_resources: &SharedResources,
    event: LambdaEvent<Request>,
) -> Result<Response, Error> {
    let request = event.payload;

    let table_name =
        get_table_name(shared_resources, request.resource.as_str());
    let key_name = get_key_name(request.resource.as_str());

    match request.method.as_str() {
        "GET" => {
            // handle cases where the record_id is not provided (e.g. GET /streams)
            if request.record_id.is_none() {
                let response =
                    list_records(shared_resources, table_name, &request.query)
                        .await;
                return response;
            }

            get_record(
                shared_resources,
                table_name,
                key_name,
                request.record_id.as_ref().unwrap(),
            )
            .await
        }
        "POST" => {
            if request.record_id.is_some() {
                return Ok(Response {
                    payload: serde_json::json!({
                        "message": "record_id should not be provided for POST requests"
                    }),
                    headers: HashMap::new(),
                    status_code: 400,
                });
            }

            create_record(
                shared_resources,
                table_name,
                request.payload.as_ref().unwrap(),
            )
            .await
        }
        "PUT" => {
            if request.record_id.is_none() {
                return Ok(Response {
                    payload: serde_json::json!({
                        "message": "record_id should be provided for PUT requests"
                    }),
                    headers: HashMap::new(),
                    status_code: 400,
                });
            }

            update_record(
                shared_resources,
                table_name,
                key_name,
                request.record_id.as_ref().unwrap(),
                request.payload.as_ref().unwrap(),
            )
            .await
        }
        "DELETE" => {
            if request.record_id.is_none() {
                return Ok(Response {
                    payload: serde_json::json!({
                        "message": "record_id should be provided for DELETE requests"
                    }),
                    headers: HashMap::new(),
                    status_code: 400,
                });
            }

            delete_record(
                shared_resources,
                table_name,
                key_name,
                request.record_id.as_ref().unwrap(),
            )
            .await
        }
        _ => panic!("unsupported method: {}", request.method),
    }
}

fn get_table_name<'a>(
    shared_resources: &'a SharedResources,
    resource: &'a str,
) -> &'a str {
    match resource {
        "streams" => &shared_resources.config.streams_table,
        "episodes" => &shared_resources.config.episodes_table,
        "series" => &shared_resources.config.series_table,
        "video_clips" => &shared_resources.config.video_metadata_table,
        _ => panic!("unsupported resource: {resource}"),
    }
}

fn get_key_name(resource: &str) -> &str {
    match resource {
        "video_clips" => "key",
        _ => "id",
    }
}

/// Lists records from the specified ``DynamoDB`` table based on the provided
/// query parameters.
///
/// # Arguments
///
/// * `shared_resources` - A reference to the shared resources containing the
///   ``DynamoDB`` client and configuration.
/// * `table_name` - The name of the ``DynamoDB`` table to scan.
/// * `query` - A hashmap containing the query parameters, including filters as
///   a JSON string.
///
/// # Returns
///
/// A `Result` containing a `Response` with the scanned items and the total
/// count, or an `Error`.
async fn list_records(
    shared_resources: &SharedResources,
    table_name: &str,
    query: &HashMap<String, String>,
) -> Result<Response, Error> {
    // Parse the query parameters
    let filters: HashMap<_, _> = match serde_json::from_str(
        query.get("filter").unwrap_or(&String::new()),
    ) {
        Ok(params) => params,
        Err(e) => {
            return Ok(Response {
                payload: serde_json::json!({
                    "message": format!("failed to parse query parameters: {}", e)
                }),
                headers: HashMap::new(),
                status_code: 400,
            });
        }
    };

    // Call the `list` function from the `dynamodb` module
    match dynamodb::list(
        &shared_resources.dynamodb,
        table_name,
        filters,
        dynamodb::PageOptions {
            cursor: None,
            limit: 10,
        },
    )
    .await
    {
        Ok(list_result) => {
            // Create the response payload
            let payload = json!({
                "items": list_result.items,
                "cursor": list_result.cursor,
            });

            // Build the response
            let response = Response {
                payload,
                headers: HashMap::new(),
                status_code: 200,
            };

            Ok(response)
        }
        Err(e) => Err(e),
    }
}

async fn get_record(
    shared_resources: &SharedResources,
    table_name: &str,
    key_name: &str,
    record_id: &str,
) -> Result<Response, Error> {
    match dynamodb::get(
        &shared_resources.dynamodb,
        table_name,
        key_name,
        record_id,
    )
    .await
    {
        Ok(result) => result.0.map_or_else(
            || {
                Ok(Response {
                    payload: serde_json::json!({}),
                    headers: HashMap::new(),
                    status_code: 404,
                })
            },
            |record| {
                Ok(Response {
                    payload: record,
                    headers: HashMap::new(),
                    status_code: 200,
                })
            },
        ),
        Err(e) => Err(e),
    }
}

async fn create_record(
    shared_resources: &SharedResources,
    table_name: &str,
    payload: &str,
) -> Result<Response, Error> {
    let parsed_payload: serde_json::Value = serde_json::from_str(payload)
        .map_err(|e| Error::from(format!("failed to parse payload: {e}")))?;

    dynamodb::create(&shared_resources.dynamodb, table_name, &parsed_payload)
        .await?;

    let response = Response {
        payload: parsed_payload,
        headers: HashMap::new(),
        status_code: 201,
    };

    Ok(response)
}

async fn update_record(
    shared_resources: &SharedResources,
    table_name: &str,
    key_name: &str,
    record_id: &str,
    payload: &str,
) -> Result<Response, Error> {
    let parsed_payload: serde_json::Value = serde_json::from_str(payload)
        .map_err(|e| Error::from(format!("failed to parse payload: {e}")))?;

    dynamodb::update(
        &shared_resources.dynamodb,
        table_name,
        key_name,
        record_id,
        &parsed_payload,
    )
    .await?;

    let response = Response {
        payload: parsed_payload,
        headers: HashMap::new(),
        status_code: 200,
    };

    Ok(response)
}

async fn delete_record(
    shared_resources: &SharedResources,
    table_name: &str,
    key_name: &str,
    record_id: &str,
) -> Result<Response, Error> {
    dynamodb::delete(
        &shared_resources.dynamodb,
        table_name,
        key_name,
        record_id,
    )
    .await?;

    let response = Response {
        payload: serde_json::json!({}),
        headers: HashMap::new(),
        // 204 No Content
        status_code: 204,
    };

    Ok(response)
}
