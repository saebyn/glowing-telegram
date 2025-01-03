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
    pub end: String,

    pub start: String,
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
pub struct Series {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub category: Option<i64>,

    pub created_at: String,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub end_date: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub end_time: Option<String>,

    pub id: String,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_active: Option<bool>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_episode_order_index: Option<i64>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub notify_subscribers: Option<bool>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub playlist_id: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub prep_notes: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub recurrence: Option<Recurrence>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub skips: Option<Vec<Skip>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub start_date: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub start_time: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub stream_count: Option<i64>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub stream_title_template: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub tags: Option<Vec<String>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub thumbnail_url: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub timezone: Option<String>,

    pub title: String,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub twitch_category: Option<TwitchCategory>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub updated_at: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Recurrence {
    pub days: Vec<Day>,

    pub interval: i64,

    #[serde(rename = "type")]
    pub recurrence_type: Type,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Day {
    Friday,

    Monday,

    Saturday,

    Sunday,

    Thursday,

    Tuesday,

    Wednesday,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Type {
    Weekly,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Skip {
    pub date: String,

    pub reason: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TwitchCategory {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub box_art_url: Option<String>,

    pub id: String,

    pub name: String,
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
    pub video_clip_count: Option<i64>,
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
    /// The path to the audio file extracted from the video clip.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub audio: Option<String>,

    pub id: String,

    /// The S3 key of the video clip.
    pub key: String,

    /// A list of paths to images that are keyframes in the video clip.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub keyframes: Option<Vec<String>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<Metadata>,

    /// The list of detected silence intervals in the video clip.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub silence: Option<Vec<Silence>>,

    /// The start time of the video clip in the context of the stream in seconds.
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
    /// The duration of the video clip in seconds.
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
    pub description: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub reasoning: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub timestamp_end: Option<f64>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub timestamp_start: Option<f64>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Highlight {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub reasoning: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub timestamp_end: Option<f64>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub timestamp_start: Option<f64>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TranscriptionError {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub reasoning: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub timestamp_start: Option<f64>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Transcription {
    pub language: String,

    pub segments: Vec<TranscriptSegment>,

    pub text: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TranscriptSegment {
    pub avg_logprob: f64,

    pub compression_ratio: f64,

    pub end: f64,

    pub no_speech_prob: f64,

    pub start: f64,

    pub temperature: f64,

    pub text: String,

    pub tokens: Vec<f64>,
}
