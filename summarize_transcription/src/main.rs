/**
 * This is the main entrypoint for the `summarize_transcription` lambda function.
 *
 * The function is responsible for summarizing the transcription of an audio file using the `OpenAI` API and saving the result to `DynamoDB`.
 */
use aws_config::{meta::region::RegionProviderChain, BehaviorVersion};
use aws_sdk_dynamodb::types::AttributeValue;
use figment::Figment;
use lambda_runtime::{service_fn, Error, LambdaEvent};
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
) -> Result<Response, Error> {
    let payload = event.payload;
    let config = &shared_resources.config;

    let transcription: Vec<TranscriptionSegment> =
        serde_dynamo::from_attribute_value(payload.transcription)
            .expect("failed to deserialize transcription");

    // Get the openai api key from secrets manager
    let openai_secret = shared_resources
        .secrets_manager
        .get_secret_value()
        .secret_id(&shared_resources.config.openai_secret_arn)
        .send()
        .await
        .expect("failed to get secret")
        .secret_string
        .expect("secret not found");

    // Call the openai api with the transcription result and summarization_context
    let openai_client = Client::new(openai_secret);

    let parameters = ChatCompletionParametersBuilder::default()
        .model(config.openai_model.clone())
        .response_format(ChatCompletionResponseFormat::JsonSchema(
            serde_json::from_str(RESPONSE_JSON_SCHEMA)
                .expect("failed to parse json schema"),
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
                    .expect("failed to serialize input"),
                ),
            },
        ])
        .build()
        .expect("failed to build chat parameters");

    let response = openai_client
        .chat()
        .create(parameters)
        .await
        .expect("failed to call openai api");

    println!("response: {:?}", response);
    println!("choices: {:?}", response.choices);

    let result = SummarizationOutput {
        summary_context: "summary_context".to_string(),
        summary_main_discussion: "summary_main_discussion".to_string(),
        title: "title".to_string(),
        keywords: vec!["keyword1".to_string(), "keyword2".to_string()],
        highlights: vec![SummaryHighlight {
            timestamp_start: 0.0,
            timestamp_end: 1.0,
            description: "description".to_string(),
            reasoning: "reasoning".to_string(),
        }],
        attentions: vec![SummaryHighlight {
            timestamp_start: 0.0,
            timestamp_end: 1.0,
            description: "description".to_string(),
            reasoning: "reasoning".to_string(),
        }],
        transcription_errors: vec![SummaryTranscriptionError {
            timestamp_start: 0.0,
            description: "description".to_string(),
            reasoning: "reasoning".to_string(),
        }],
    };

    let summarization_context = result.summary_context.clone();

    shared_resources
        .dynamodb
        .update_item()
        .table_name(&shared_resources.config.metadata_table_name)
        .key("key", AttributeValue::S(payload.input_key))
        .update_expression("SET #summary = :summary")
        .expression_attribute_names("#summary", "summary")
        .expression_attribute_values(":summary", result.into())
        .send()
        .await
        .expect("failed to save result");

    Ok(Response {
        summarization_context,
        transcription_context: payload.transcription_context,
    })
}
