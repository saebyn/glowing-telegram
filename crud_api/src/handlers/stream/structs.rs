use serde::{Deserialize, Serialize};

use crate::models::Stream;

#[derive(Debug, Deserialize)]
pub struct CreateStreamRequest {
    pub title: String,
    pub description: Option<String>,
    pub thumbnail: Option<String>,
    pub topic_ids: Option<Vec<i32>>,
    pub prefix: String,
    pub speech_audio_track: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateStreamRequest {
    pub title: Option<String>,
    pub description: Option<String>,
    pub thumbnail: Option<String>,
    pub topic_ids: Option<Vec<i32>>,
    pub prefix: Option<String>,
    pub speech_audio_track: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct StreamDetailView {
    pub id: String,
    pub title: String,
    pub description: String,
    pub thumbnail: String,
    pub created_at: String,
    pub updated_at: Option<String>,
    pub topic_ids: Vec<String>,
}

#[derive(Debug, Serialize)]
pub struct StreamSimpleView {
    pub id: String,
    pub title: String,
    pub thumbnail: String,
    pub created_at: String,
    pub updated_at: Option<String>,
    pub topic_ids: Vec<String>,
}

impl From<Stream> for StreamSimpleView {
    fn from(stream: Stream) -> Self {
        StreamSimpleView {
            id: stream.id.to_string(),
            title: stream.title.to_string(),
            thumbnail: stream.thumbnail_url.to_string(),
            created_at: stream.created_at.to_string(),
            updated_at: stream.updated_at.map(|dt| dt.to_string()),
            topic_ids: vec![],
        }
    }
}

impl From<Stream> for StreamDetailView {
    fn from(stream: Stream) -> Self {
        StreamDetailView {
            id: stream.id.to_string(),
            title: stream.title.to_string(),
            description: stream.description.to_string(),
            thumbnail: stream.thumbnail_url.to_string(),
            created_at: stream.created_at.to_string(),
            updated_at: stream.updated_at.map(|dt| dt.to_string()),
            topic_ids: vec![],
        }
    }
}

fn deserialize_range<'de, D>(deserializer: D) -> Result<Option<(i64, i64)>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    use serde::de::Error;

    let s = String::deserialize(deserializer)?;

    let range = s
        .trim_start_matches('[')
        .trim_end_matches(']')
        .split(',')
        .map(|s| s.parse::<i64>())
        .collect::<Result<Vec<i64>, _>>()
        .map_err(|_| Error::custom("invalid range"))?;

    if range.len() != 2 {
        return Err(Error::custom("invalid range"));
    }

    Ok(Some((range[0], range[1])))
}

/**
 * Params for list endpoint for ra-data-simple-rest
 *
 * @see https://marmelab.com/react-admin/DataProviders.html#rest-api-parameters
 */
#[derive(Debug, Deserialize)]
pub struct ListParams {
    // Pagination
    // query string has the following format: range=[0, 24]
    // convert this string to a tuple
    #[serde(deserialize_with = "deserialize_range")]
    pub range: Option<(i64, i64)>,

    // Sort
    pub sort: Option<String>,
    pub order: Option<String>,
}
