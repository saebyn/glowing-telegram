/**
 * This is the main entrypoint for the `crud_api` lambda function.
 *
 * The function is responsible for handling the requests and responses for the
 * CRUD operations, in a way compatible with the ra-data-simple-rest data
 * provider for React Admin.
 *
 */
use aws_config::{meta::region::RegionProviderChain, BehaviorVersion};
use aws_lambda_events::{
    apigw::{ApiGatewayProxyRequest, ApiGatewayProxyResponse},
    http::HeaderMap,
    query_map::QueryMap,
};
use aws_sdk_dynamodb::Client;
use figment::Figment;
use lambda_runtime::{service_fn, Error, LambdaEvent};
use serde::Deserialize;
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
    profiles_table: String,
}

fn load_config() -> Result<Config, figment::Error> {
    let figment = Figment::new().merge(figment::providers::Env::raw());

    figment.extract()
}

#[derive(Debug, Clone)]
struct Response {
    payload: serde_json::Value,
    headers: HeaderMap,
    status_code: i64,
}

#[derive(Debug)]
struct SharedResources {
    dynamodb: Client,
    config: Config,
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

    let func = service_fn(
        move |event: LambdaEvent<ApiGatewayProxyRequest>| async move {
            handler(shared_resources, event).await
        },
    );

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

    lambda_runtime::run(func).await?;

    Ok(())
}

#[tracing::instrument(skip(event, shared_resources), fields(req_id = %event.context.request_id))]
async fn handler(
    shared_resources: &SharedResources,
    event: LambdaEvent<ApiGatewayProxyRequest>,
) -> Result<ApiGatewayProxyResponse, Error> {
    let request = event.payload;

    tracing::info!("received request: {:?}", request);

    let path = request.path_parameters.get("proxy");

    let (record_type, record_id) = match path {
        Some(path) => {
            let parts: Vec<&str> = path.split('/').collect();
            if parts.len() == 1 {
                (parts[0], None)
            } else if parts.len() == 2 {
                (parts[0], Some(parts[1]))
            } else {
                return Ok(ApiGatewayProxyResponse {
                    status_code: 400,
                    headers: HeaderMap::new(),
                    body: Some(aws_lambda_events::encodings::Body::Text(
                        "Invalid path".to_string(),
                    )),
                    is_base64_encoded: false,
                    ..Default::default()
                });
            }
        }
        None => {
            return Ok(ApiGatewayProxyResponse {
                status_code: 400,
                headers: HeaderMap::new(),
                body: Some(aws_lambda_events::encodings::Body::Text(
                    "Invalid path".to_string(),
                )),
                is_base64_encoded: false,
                ..Default::default()
            });
        }
    };

    let table_name = get_table_name(shared_resources, record_type);
    let key_name = get_key_name(record_type);

    let result = match request.http_method.as_str() {
        "GET" => {
            // handle cases where the record_id is not provided (e.g. GET /streams)
            if record_id.is_none() {
                list_records(
                    shared_resources,
                    table_name,
                    &request.query_string_parameters,
                )
                .await
            } else {
                get_record(
                    shared_resources,
                    table_name,
                    key_name,
                    record_id.unwrap(),
                )
                .await
            }
        }
        "POST" => {
            if record_id.is_some() {
                Ok(Response {
                    payload: serde_json::json!({
                        "message": "record_id should not be provided for POST requests"
                    }),
                    headers: HeaderMap::new(),
                    status_code: 400,
                })
            } else {
                create_record(
                    shared_resources,
                    table_name,
                    request.body.as_ref().unwrap(),
                )
                .await
            }
        }
        "PUT" => {
            if record_id.is_none() {
                Ok(Response {
                    payload: serde_json::json!({
                        "message": "record_id should be provided for PUT requests"
                    }),
                    headers: HeaderMap::new(),
                    status_code: 400,
                })
            } else {
                update_record(
                    shared_resources,
                    table_name,
                    key_name,
                    record_id.unwrap(),
                    request.body.as_ref().unwrap(),
                )
                .await
            }
        }
        "DELETE" => {
            if record_id.is_none() {
                Ok(Response {
                    payload: serde_json::json!({
                        "message": "record_id should be provided for DELETE requests"
                    }),
                    headers: HeaderMap::new(),
                    status_code: 400,
                })
            } else {
                delete_record(
                    shared_resources,
                    table_name,
                    key_name,
                    record_id.as_ref().unwrap(),
                )
                .await
            }
        }
        _ => panic!("unsupported method: {}", request.http_method),
    };

    let mut cors_headers = HeaderMap::new();
    cors_headers.insert("Access-Control-Allow-Origin", "*".parse().unwrap());
    cors_headers.insert(
        "Access-Control-Allow-Methods",
        "GET, POST, PUT, DELETE, OPTIONS".parse().unwrap(),
    );
    cors_headers.insert(
        "Access-Control-Allow-Headers",
        "Content-Type, Authorization".parse().unwrap(),
    );

    match result {
        Ok(mut response) => {
            tracing::info!("response: {:?}", response);

            response.headers.extend(cors_headers);

            Ok(ApiGatewayProxyResponse {
                status_code: response.status_code,
                headers: response.headers,
                body: Some(aws_lambda_events::encodings::Body::Text(
                    response.payload.to_string(),
                )),
                is_base64_encoded: false,
                ..Default::default()
            })
        }
        Err(e) => {
            tracing::error!("error: {:?}", e);
            Err(Error::from(e))
        }
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
        "profiles" => &shared_resources.config.profiles_table,
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
#[allow(clippy::option_if_let_else)]
async fn list_records(
    shared_resources: &SharedResources,
    table_name: &str,
    query: &QueryMap,
) -> Result<Response, Error> {
    tracing::info!("listing records from table: {table_name}");

    // Parse the query parameters
    let filters = match query.first("filter") {
        Some(filter) => match filter {
            "" => HashMap::new(),
            _ => match serde_json::from_str(filter) {
                Ok(filters) => filters,
                Err(e) => {
                    tracing::warn!("failed to parse filters: {e}");
                    HashMap::new()
                }
            },
        },
        None => HashMap::new(),
    };

    // Call the `list` function from the `dynamodb` module
    // TODO - handle pagination
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
                headers: HeaderMap::new(),
                status_code: 200,
            };

            tracing::info!("successfully listed records");

            Ok(response)
        }
        Err(e) => {
            tracing::error!("failed to list records: {e}");

            Err(e)
        }
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
                    headers: HeaderMap::new(),
                    status_code: 404,
                })
            },
            |record| {
                Ok(Response {
                    payload: record,
                    headers: HeaderMap::new(),
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
        headers: HeaderMap::new(),
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

    // return the updated record
    let record = match dynamodb::get(
        &shared_resources.dynamodb,
        table_name,
        key_name,
        record_id,
    )
    .await
    {
        Ok(result) => match result.0 {
            Some(record) => record,
            None => return Err(Error::from("record not found")),
        },
        Err(e) => return Err(e),
    };

    let response = Response {
        payload: record,
        headers: HeaderMap::new(),
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
        headers: HeaderMap::new(),
        // 204 No Content
        status_code: 204,
    };

    Ok(response)
}
