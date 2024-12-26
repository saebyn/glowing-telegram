// Example code that deserializes and serializes the model.
// extern crate serde;
// #[macro_use]
// extern crate serde_derive;
// extern crate serde_json;
//
// use generated_module::AccessTokenResponse;
//
// fn main() {
//     let json = r#"{"answer": 42}"#;
//     let model: AccessTokenResponse = serde_json::from_str(&json).unwrap();
// }

use serde::{Serialize, Deserialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AccessTokenResponse {
    pub access_token: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AuthorizationUrlResponse {
    pub url: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Episode {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,

    pub id: String,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub stream_id: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub tracks: Option<Vec<Track>>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Track {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub end: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub start: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct IdOnly {
    pub id: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Profile {
    pub id: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SimpleChatMessage {
    pub content: String,

    pub role: Role,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Role {
    Assistant,

    Function,

    System,

    Tool,

    User,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Stream {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub created_at: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub duration: Option<f64>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub has_episodes: Option<bool>,

    pub id: String,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub prefix: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub series_id: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub stream_date: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub stream_platform: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub thumbnail_url: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub updated_at: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub video_clip_count: Option<f64>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StreamIngestionRequest {
    pub initial_prompt: String,

    pub initial_summary: String,

    pub stream_id: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TwitchAuthRequest {
    pub redirect_uri: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TwitchCallbackRequest {
    pub code: String,

    pub scope: Vec<String>,

    pub state: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct VideoClip {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub audio: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub key: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub keyframes: Option<Vec<String>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<Metadata>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub silence: Option<Vec<Silence>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub start_time: Option<f64>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub stream_id: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub summary: Option<Summary>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub transcription: Option<Transcription>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Metadata {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub format: Option<Format>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Format {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub duration: Option<f64>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Silence {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub end: Option<f64>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub start: Option<f64>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Summary {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub attentions: Option<Vec<Attention>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub highlights: Option<Vec<Highlight>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub keywords: Option<Vec<String>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub summary_context: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub summary_main_discussion: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub transcription_errors: Option<Vec<TranscriptionError>>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Attention {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<f64>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub reasoning: Option<f64>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub timestamp_end: Option<f64>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub timestamp_start: Option<f64>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Highlight {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<f64>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub reasoning: Option<f64>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub timestamp_end: Option<f64>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub timestamp_start: Option<f64>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TranscriptionError {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<f64>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub reasoning: Option<f64>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub timestamp_start: Option<f64>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Transcription {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub language: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub segments: Option<Vec<SegmentElement>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SegmentElement {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub avg_logprob: Option<f64>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub compression_ratio: Option<f64>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub end: Option<f64>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub no_speech_prob: Option<f64>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub start: Option<f64>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f64>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
}
