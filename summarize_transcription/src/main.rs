/**
 * This is the main entrypoint for the `summarize_transcription` lambda function.
 *
 * The function is responsible for summarizing the transcription of an audio file using the `OpenAI` API and saving the result to `DynamoDB`.
 */
use aws_sdk_dynamodb::types::AttributeValue;
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

const RESPONSE_JSON_SCHEMA: &str = include_str!("response_json_schema.json");

#[derive(Debug, Deserialize)]
struct Config {
    openai_secret_arn: String,
    metadata_table_name: String,
    openai_model: String,
    openai_instructions: String,
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
struct AppContext {
    dynamodb: aws_sdk_dynamodb::Client,
    secrets_manager: aws_sdk_secretsmanager::Client,
    config: Config,
}

impl gt_app::ContextProvider<Config> for AppContext {
    async fn new(config: Config, aws_config: aws_config::SdkConfig) -> Self {
        let dynamodb = aws_sdk_dynamodb::Client::new(&aws_config);
        let secrets_manager = aws_sdk_secretsmanager::Client::new(&aws_config);

        Self {
            dynamodb,
            secrets_manager,
            config,
        }
    }
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

impl TryFrom<SummarizationOutput> for AttributeValue {
    type Error = serde_json::Error;

    fn try_from(output: SummarizationOutput) -> Result<Self, Self::Error> {
        let json = serde_json::to_value(output)?;

        Ok(types::utils::convert_json_to_attribute_value(json))
    }
}

#[tokio::main]
async fn main() -> Result<(), Error> {
    let app_context = &gt_app::create_app_context().await.unwrap();

    lambda_runtime::run(service_fn(
        move |event: LambdaEvent<Request>| async move {
            handler(app_context, event).await
        },
    ))
    .await?;
    Ok(())
}

async fn handler(
    app_context: &AppContext,
    event: LambdaEvent<Request>,
) -> Result<Response, ErrorResponse> {
    let payload = event.payload;
    let config = &app_context.config;

    let transcription: Vec<TranscriptionSegment> =
        serde_dynamo::from_attribute_value(payload.transcription)
            .or(Err(ErrorResponse("InvalidInput", "Invalid transcription")))?;

    // Get the openai api key from secrets manager
    let openai_secret = fetch_openai_secret(app_context).await?;

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

    update_transcription_summary(app_context, &payload.input_key, result)
        .await?;

    Ok(Response {
        summarization_context,
        transcription_context: payload.transcription_context,
    })
}

async fn fetch_openai_secret(
    shared_resources: &AppContext,
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
    app_context: &AppContext,
    input_key: &str,
    result: SummarizationOutput,
) -> Result<(), ErrorResponse> {
    let result = result.try_into().or(Err(ErrorResponse(
        "InvalidResponse",
        "Invalid response from OpenAI",
    )))?;

    app_context
        .dynamodb
        .update_item()
        .table_name(&app_context.config.metadata_table_name)
        .key("key", AttributeValue::S(input_key.to_string()))
        .update_expression("SET #summary = :summary")
        .expression_attribute_names("#summary", "summary")
        .expression_attribute_values(":summary", result)
        .send()
        .await
        .inspect_err(|err| {
            tracing::error!(
                "Failed to update the summary in DynamoDB: {:?}",
                err
            );
        })
        .or(Err(ErrorResponse(
            "ServerError",
            "Failed to update the summary in DynamoDB",
        )))?;

    Ok(())
}
