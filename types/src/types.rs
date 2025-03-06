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

    pub broadcaster_id: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AuthorizationUrlResponse {
    pub url: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CutList {
    /// List of input media sources
    pub input_media: Vec<InputMedia>,

    /// Ordered media sections to form the output timeline sequence
    pub output_track: Vec<OutputTrack>,

    /// One or more overlay tracks
    #[serde(skip_serializing_if = "Option::is_none")]
    pub overlay_tracks: Option<Vec<OverlayTrack>>,

    /// Schema version
    pub version: CutListVersion,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InputMedia {
    /// Path of the media
    pub s3_location: String,

    /// Start/end frames to select
    pub sections: Vec<MediaSection>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MediaSection {
    /// End frame is exclusive
    pub end_frame: i64,

    pub start_frame: i64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OutputTrack {
    /// Index of the media source
    pub media_index: i64,

    /// Index of the section in the media source
    pub section_index: i64,

    /// Transition to apply at the start of the section
    #[serde(skip_serializing_if = "Option::is_none")]
    pub transition_in: Option<TransitionInClass>,

    /// Transition to apply at the end of the section
    #[serde(skip_serializing_if = "Option::is_none")]
    pub transition_out: Option<TransitionOutClass>,
}

/// Transition to apply at the start of the section
///
/// Transition to apply at the start or end of a media section
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TransitionInClass {
    /// Duration of the transition in frames, relative to the start/end of the section
    pub duration: i64,

    /// Transition type
    #[serde(rename = "type")]
    pub transition_type: TransitionInType,
}

/// Transition type
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TransitionInType {
    Cut,

    Fade,
}

/// Transition to apply at the end of the section
///
/// Transition to apply at the start of the section
///
/// Transition to apply at the start or end of a media section
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TransitionOutClass {
    /// Duration of the transition in frames, relative to the start/end of the section
    pub duration: i64,

    /// Transition type
    #[serde(rename = "type")]
    pub transition_type: TransitionInType,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OverlayTrack {
    /// Index of the media source
    pub media_index: i64,

    /// Index of the section in the media source
    pub section_index: i64,

    /// Start frame on the overlay track
    pub start_frame: i64,

    /// Overlay type
    #[serde(rename = "type")]
    pub overlay_track_type: OverlayTrackType,

    /// X position of the overlay
    #[serde(skip_serializing_if = "Option::is_none")]
    pub x: Option<f64>,

    /// Y position of the overlay
    #[serde(skip_serializing_if = "Option::is_none")]
    pub y: Option<f64>,
}

/// Overlay type
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OverlayTrackType {
    Alpha,

    Colorkey,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum CutListVersion {
    #[serde(rename = "1.0.0")]
    The100,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Episode {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub category: Option<i64>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub created_at: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub cut_list: Option<CutListClass>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub error_message: Option<String>,

    pub id: String,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_published: Option<bool>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub notify_subscribers: Option<bool>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub order_index: Option<i64>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub render_uri: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub retry_after_seconds: Option<i64>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub stream_id: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub tags: Option<Vec<String>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub tracks: Option<Vec<Track>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub updated_at: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub upload_attempts: Option<i64>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub upload_resume_at_byte: Option<i64>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub upload_status: Option<UploadStatus>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub user_id: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub youtube_upload_url: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub youtube_video_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CutListClass {
    /// List of input media sources
    pub input_media: Vec<InputMedia>,

    /// Ordered media sections to form the output timeline sequence
    pub output_track: Vec<OutputTrack>,

    /// One or more overlay tracks
    #[serde(skip_serializing_if = "Option::is_none")]
    pub overlay_tracks: Option<Vec<OverlayTrack>>,

    /// Schema version
    pub version: CutListVersion,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Track {
    pub end: String,

    pub start: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum UploadStatus {
    #[serde(rename = "FAILED")]
    Failed,

    #[serde(rename = "SUCCESS")]
    Success,

    #[serde(rename = "THROTTLED")]
    Throttled,
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
#[serde(rename_all = "camelCase")]
pub struct RenderRequest {
    pub episode_ids: Vec<String>,
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
    pub recurrence_type: RecurrenceType,
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
pub enum RecurrenceType {
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

    pub scopes: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TwitchCallbackRequest {
    pub code: String,

    pub scope: Vec<String>,

    pub state: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TwitchCallbackResponse {
    /// The URL to redirect the client to after the authorization flow is complete.
    pub url: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TwitchSessionSecret {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub access_token: Option<String>,

    pub csrf_token: String,

    pub redirect_url: String,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub refresh_token: Option<String>,

    pub scopes: Vec<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub valid_until: Option<f64>,
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

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct YouTubeAuthRequest {
    pub redirect_uri: String,

    pub scopes: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct YouTubeCallbackRequest {
    pub code: String,

    pub scope: Vec<String>,

    pub state: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct YouTubeCallbackResponse {
    /// The URL to redirect the client to after the authorization flow is complete.
    pub url: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct YouTubeSessionSecret {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub access_token: Option<String>,

    pub csrf_token: String,

    pub redirect_url: String,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub refresh_token: Option<String>,

    pub scopes: Vec<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub valid_until: Option<f64>,
}
