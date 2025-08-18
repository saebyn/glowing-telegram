use aws_sdk_dynamodb::types::AttributeValue;
use std::collections::HashMap;
use thiserror::Error;
use types::{Silence, Transcription};

#[derive(Error, Debug)]
pub enum DynamoDbError {
    #[error("AWS SDK error: {0}")]
    AwsSdkError(
        #[from]
        aws_sdk_dynamodb::error::SdkError<
            aws_sdk_dynamodb::operation::get_item::GetItemError,
        >,
    ),
    #[error("Item not found in DynamoDB")]
    ItemNotFound,
    #[error("No silence data found")]
    NoSilenceData,
    #[error("No metadata found")]
    NoMetadata,
    #[error("No format data found")]
    NoFormatData,
    #[error("No duration found")]
    NoDuration,
    #[error("Duration is not a number")]
    InvalidDuration,
    #[error("Format is not a map")]
    InvalidFormat,
    #[error("Metadata is not a map")]
    InvalidMetadata,
    #[error("Failed to parse number: {0}")]
    ParseError(#[from] std::num::ParseFloatError),
}

pub async fn get_item_from_dynamodb(
    dynamodb: &aws_sdk_dynamodb::Client,
    table_name: &str,
    item_key: &str,
) -> Result<HashMap<String, AttributeValue>, DynamoDbError> {
    let response = dynamodb
        .get_item()
        .table_name(table_name)
        .key("key", AttributeValue::S(item_key.to_string()))
        .send()
        .await?;

    response.item.ok_or(DynamoDbError::ItemNotFound)
}

pub fn get_silence_data_from_item(
    item: &HashMap<String, AttributeValue>,
) -> Result<Vec<Silence>, DynamoDbError> {
    let silence_attr =
        item.get("silence").ok_or(DynamoDbError::NoSilenceData)?;

    let AttributeValue::L(silence_list) = silence_attr else {
        return Ok(Vec::new());
    };

    let mut silence_segments = Vec::new();

    for segment_attr in silence_list {
        let AttributeValue::M(segment_map) = segment_attr else {
            continue;
        };

        let Some(AttributeValue::N(start_str)) = segment_map.get("start")
        else {
            continue;
        };

        let Some(AttributeValue::N(end_str)) = segment_map.get("end") else {
            continue;
        };

        let start = start_str.parse::<f64>()?;
        let end = end_str.parse::<f64>()?;

        silence_segments.push(Silence {
            start: Some(start),
            end: Some(end),
        });
    }

    Ok(silence_segments)
}

pub fn get_duration_from_item(
    item: &HashMap<String, AttributeValue>,
) -> Result<f64, DynamoDbError> {
    let metadata_attr =
        item.get("metadata").ok_or(DynamoDbError::NoMetadata)?;

    let AttributeValue::M(metadata_map) = metadata_attr else {
        return Err(DynamoDbError::InvalidMetadata);
    };

    let format_attr = metadata_map
        .get("format")
        .ok_or(DynamoDbError::NoFormatData)?;

    let AttributeValue::M(format_map) = format_attr else {
        return Err(DynamoDbError::InvalidFormat);
    };

    let duration_attr = format_map
        .get("duration")
        .ok_or(DynamoDbError::NoDuration)?;

    let AttributeValue::N(duration_str) = duration_attr else {
        return Err(DynamoDbError::InvalidDuration);
    };

    Ok(duration_str.parse::<f64>()?)
}

pub fn convert_transcription_to_attributevalue(
    transcription: Transcription,
) -> AttributeValue {
    let segments = transcription
        .segments
        .iter()
        .map(|segment| {
            let mut map = HashMap::new();

            map.insert(
                "start".to_string(),
                AttributeValue::N(segment.start.to_string()),
            );
            map.insert(
                "end".to_string(),
                AttributeValue::N(segment.end.to_string()),
            );
            map.insert(
                "text".to_string(),
                AttributeValue::S(segment.text.clone()),
            );
            map.insert(
                "tokens".to_string(),
                AttributeValue::L(
                    segment
                        .tokens
                        .iter()
                        .map(|token| AttributeValue::N(token.to_string()))
                        .collect(),
                ),
            );
            map.insert(
                "temperature".to_string(),
                AttributeValue::N(segment.temperature.to_string()),
            );
            map.insert(
                "avg_logprob".to_string(),
                AttributeValue::N(segment.avg_logprob.to_string()),
            );
            map.insert(
                "compression_ratio".to_string(),
                AttributeValue::N(segment.compression_ratio.to_string()),
            );
            map.insert(
                "no_speech_prob".to_string(),
                AttributeValue::N(segment.no_speech_prob.to_string()),
            );

            AttributeValue::M(map)
        })
        .collect();

    AttributeValue::M(
        vec![
            ("text".to_string(), AttributeValue::S(transcription.text)),
            ("segments".to_string(), AttributeValue::L(segments)),
            (
                "language".to_string(),
                AttributeValue::S(transcription.language),
            ),
        ]
        .into_iter()
        .collect(),
    )
}
