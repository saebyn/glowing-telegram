/**
 * This is the main entrypoint for the `summarize_transcription` lambda function.
 *
 * The function is responsible for summarizing the transcription of an audio file using the `OpenAI` API and saving the result to `DynamoDB`.
 */
use aws_config::{BehaviorVersion, meta::region::RegionProviderChain};
use aws_sdk_dynamodb::types::AttributeValue;
use figment::Figment;
use lambda_runtime::{Diagnostic, Error, LambdaEvent, service_fn};
use openai_dive::v1::error::APIError;
use openai_dive::v1::resources::shared::FinishReason::StopSequenceReached;
use openai_dive::v1::{
    api::Client,
    resources::chat::{
        ChatCompletionParametersBuilder, ChatCompletionResponseFormat,
        ChatMessage, ChatMessageContent,
    },
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

const RESPONSE_JSON_SCHEMA: &str = include_str!("response_json_schema.json");

#[derive(Debug, Deserialize)]
struct Config {
    openai_secret_arn: String,
    metadata_table_name: String,
    openai_model: String,
    openai_instructions: String,
}

fn load_config() -> Result<Config, figment::Error> {
    let figment = Figment::new().merge(figment::providers::Env::raw());

    figment.extract()
}

#[derive(Debug, Deserialize, Serialize)]
struct TranscriptionSegment {
    start: f64,
    end: f64,
    text: String,
}

#[derive(Deserialize, Debug)]
struct Request {
    input_key: String,
    transcription: serde_dynamo::AttributeValue,
    transcription_context: String,
    summarization_context: String,
}

#[derive(Serialize)]
struct Response {
    transcription_context: String,
    summarization_context: String,
}

#[derive(Debug)]
struct ErrorResponse(&'static str, &'static str);

impl From<ErrorResponse> for Diagnostic {
    fn from(error: ErrorResponse) -> Self {
        Self {
            error_type: error.0.to_string(),
            error_message: error.1.to_string(),
        }
    }
}

#[derive(Debug)]
struct SharedResources {
    dynamodb: aws_sdk_dynamodb::Client,
    secrets_manager: aws_sdk_secretsmanager::Client,
    config: Config,
}

#[derive(Debug, Serialize)]
struct SummarizationInput {
    transcription: Vec<TranscriptionSegment>,
    summarization_context: String,
}

#[derive(Debug, Deserialize, Serialize)]
struct SummaryHighlight {
    timestamp_start: f64,
    timestamp_end: f64,
    description: String,
    reasoning: String,
}

#[derive(Debug, Deserialize, Serialize)]
struct SummaryTranscriptionError {
    timestamp_start: f64,
    description: String,
    reasoning: String,
}

#[derive(Debug, Deserialize, Serialize)]
struct SummarizationOutput {
    summary_context: String,
    summary_main_discussion: String,
    title: String,
    keywords: Vec<String>,
    highlights: Vec<SummaryHighlight>,
    attentions: Vec<SummaryHighlight>,
    transcription_errors: Vec<SummaryTranscriptionError>,
}

impl From<SummarizationOutput> for AttributeValue {
    fn from(output: SummarizationOutput) -> Self {
        let mut map = HashMap::new();
        map.insert(
            "summary_context".to_string(),
            AttributeValue::S(output.summary_context),
        );
        map.insert(
            "summary_main_discussion".to_string(),
            AttributeValue::S(output.summary_main_discussion),
        );
        map.insert("title".to_string(), AttributeValue::S(output.title));
        map.insert(
            "keywords".to_string(),
            AttributeValue::Ss(output.keywords),
        );
        map.insert(
            "highlights".to_string(),
            AttributeValue::L(
                output
                    .highlights
                    .iter()
                    .map(|highlight| {
                        AttributeValue::M(
                            vec![
                                (
                                    "timestamp_start".to_string(),
                                    AttributeValue::N(
                                        highlight.timestamp_start.to_string(),
                                    ),
                                ),
                                (
                                    "timestamp_end".to_string(),
                                    AttributeValue::N(
                                        highlight.timestamp_end.to_string(),
                                    ),
                                ),
                                (
                                    "description".to_string(),
                                    AttributeValue::S(
                                        highlight.description.clone(),
                                    ),
                                ),
                                (
                                    "reasoning".to_string(),
                                    AttributeValue::S(
                                        highlight.reasoning.clone(),
                                    ),
                                ),
                            ]
                            .into_iter()
                            .collect(),
                        )
                    })
                    .collect(),
            ),
        );
        map.insert(
            "attentions".to_string(),
            AttributeValue::L(
                output
                    .attentions
                    .iter()
                    .map(|attention| {
                        AttributeValue::M(
                            vec![
                                (
                                    "timestamp_start".to_string(),
                                    AttributeValue::N(
                                        attention.timestamp_start.to_string(),
                                    ),
                                ),
                                (
                                    "timestamp_end".to_string(),
                                    AttributeValue::N(
                                        attention.timestamp_end.to_string(),
                                    ),
                                ),
                                (
                                    "description".to_string(),
                                    AttributeValue::S(
                                        attention.description.clone(),
                                    ),
                                ),
                                (
                                    "reasoning".to_string(),
                                    AttributeValue::S(
                                        attention.reasoning.clone(),
                                    ),
                                ),
                            ]
                            .into_iter()
                            .collect(),
                        )
                    })
                    .collect(),
            ),
        );
        map.insert(
            "transcription_errors".to_string(),
            AttributeValue::L(
                output
                    .transcription_errors
                    .iter()
                    .map(|error| {
                        AttributeValue::M(
                            vec![
                                (
                                    "timestamp_start".to_string(),
                                    AttributeValue::N(
                                        error.timestamp_start.to_string(),
                                    ),
                                ),
                                (
                                    "description".to_string(),
                                    AttributeValue::S(
                                        error.description.clone(),
                                    ),
                                ),
                                (
                                    "reasoning".to_string(),
                                    AttributeValue::S(error.reasoning.clone()),
                                ),
                            ]
                            .into_iter()
                            .collect(),
                        )
                    })
                    .collect(),
            ),
        );

        Self::M(map)
    }
}

#[tokio::main]
async fn main() -> Result<(), Error> {
    // https://docs.aws.amazon.com/lambda/latest/dg/rust-logging.html
    tracing_subscriber::fmt()
        .json()
        // allow log level to be overridden by RUST_LOG env var
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
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

    let config = load_config().expect("failed to load config");
    let region_provider =
        RegionProviderChain::default_provider().or_else("us-east-1");
    let aws_config = aws_config::defaults(BehaviorVersion::latest())
        .region(region_provider)
        .load()
        .await;

    let dynamodb = aws_sdk_dynamodb::Client::new(&aws_config);
    let secrets_manager = aws_sdk_secretsmanager::Client::new(&aws_config);

    let shared_resources = &SharedResources {
        dynamodb,
        secrets_manager,
        config,
    };

    lambda_runtime::run(service_fn(
        move |event: LambdaEvent<Request>| async move {
            handler(shared_resources, event).await
        },
    ))
    .await?;
    Ok(())
}

async fn handler(
    shared_resources: &SharedResources,
    event: LambdaEvent<Request>,
) -> Result<Response, ErrorResponse> {
    let payload = event.payload;
    let config = &shared_resources.config;

    let transcription: Vec<TranscriptionSegment> =
        serde_dynamo::from_attribute_value(payload.transcription)
            .or(Err(ErrorResponse("InvalidInput", "Invalid transcription")))?;

    // Get the openai api key from secrets manager
    let openai_secret = fetch_openai_secret(shared_resources).await?;

    // Call the openai api with the transcription result and summarization_context
    let openai_client = Client::new(openai_secret);

    let parameters = ChatCompletionParametersBuilder::default()
        .model(config.openai_model.clone())
        .response_format(ChatCompletionResponseFormat::JsonSchema(
            serde_json::from_str(RESPONSE_JSON_SCHEMA).or(Err(
                ErrorResponse(
                    "InvalidResponseSchema",
                    "Invalid response schema from OpenAI",
                ),
            ))?,
        ))
        .messages(vec![
            ChatMessage::System {
                name: None,
                content: ChatMessageContent::Text(
                    config.openai_instructions.clone(),
                ),
            },
            ChatMessage::User {
                name: None,
                content: ChatMessageContent::Text(
                    serde_json::to_string(&SummarizationInput {
                        transcription,
                        summarization_context: payload.summarization_context,
                    })
                    .or(Err(ErrorResponse(
                        "InvalidInput",
                        "Invalid input for summarization task",
                    )))?,
                ),
            },
        ])
        .build()
        .or(Err(ErrorResponse(
            "InvalidInput",
            "Invalid input for summarization task parameters: cannot build parameters",
        )))?;

    let response = match openai_client.chat().create(parameters).await {
        Ok(response) => response,
        Err(APIError::RateLimitError(message)) => {
            tracing::warn!("Rate limit reached: {}", message);
            return Err(ErrorResponse("RateLimitError", "Rate limit reached"));
        }
        Err(err) => {
            tracing::error!("Failed to complete chat: {:?}", err);
            return Err(ErrorResponse(
                "ServerError",
                "Failed to call OpenAI API",
            ));
        }
    };

    // Check that we got a choice from the openai api
    let choice = response.choices.into_iter().next().ok_or(ErrorResponse(
        "InvalidResponse",
        "Invalid response from OpenAI",
    ))?;

    // Check that the finish_reason is StopSequenceReached
    assert_eq!(choice.finish_reason.unwrap(), StopSequenceReached);

    // get the assistant's response
    let ChatMessage::Assistant {
        content: Some(ChatMessageContent::Text(text)),
        ..
    } = choice.message
    else {
        return Err(ErrorResponse(
            "InvalidResponse",
            "Invalid response from OpenAI",
        ));
    };

    // Parse the result as a SummarizationOutput
    let result =
        serde_json::from_str::<SummarizationOutput>(&text).or(Err(
            ErrorResponse("InvalidResponse", "Invalid response from OpenAI"),
        ))?;

    let summarization_context = result.summary_context.clone();

    update_transcription_summary(shared_resources, &payload.input_key, result)
        .await?;

    Ok(Response {
        summarization_context,
        transcription_context: payload.transcription_context,
    })
}

async fn fetch_openai_secret(
    shared_resources: &SharedResources,
) -> Result<String, ErrorResponse> {
    let openai_secret = shared_resources
        .secrets_manager
        .get_secret_value()
        .secret_id(&shared_resources.config.openai_secret_arn)
        .send()
        .await
        .or(Err(ErrorResponse("SecretNotFound", "Secret not found")))?
        .secret_string
        .ok_or(ErrorResponse("SecretNotFound", "Secret not found"))?;
    Ok(openai_secret)
}

async fn update_transcription_summary(
    shared_resources: &SharedResources,
    input_key: &str,
    result: SummarizationOutput,
) -> Result<(), ErrorResponse> {
    shared_resources
        .dynamodb
        .update_item()
        .table_name(&shared_resources.config.metadata_table_name)
        .key("key", AttributeValue::S(input_key.to_string()))
        .update_expression("SET #summary = :summary")
        .expression_attribute_names("#summary", "summary")
        .expression_attribute_values(":summary", result.into())
        .send()
        .await
        .or(Err(ErrorResponse(
            "ServerError",
            "Failed to update the summary in DynamoDB",
        )))?;

    Ok(())
}
