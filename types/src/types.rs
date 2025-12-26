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
use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AccessTokenResponse {
    pub access_token: String,

    pub broadcaster_id: String,

    pub login: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AuthorizationUrlResponse {
    pub url: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ChatSubscriptionStatusResponse {
    /// Whether the user has any active chat subscriptions
    pub has_active_subscription: bool,

    /// Array of active EventSub chat subscriptions for the user
    pub subscriptions: Vec<EventSubSubscription>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EventSubSubscription {
    /// The condition object for the subscription
    pub condition: Condition,

    /// When the subscription was created
    pub created_at: String,

    /// The subscription ID
    pub id: String,

    /// The status of the subscription
    pub status: String,

    /// The transport object for the subscription
    pub transport: Transport,

    /// The type of the subscription
    #[serde(rename = "type")]
    pub event_sub_subscription_type: String,

    /// The version of the subscription
    pub version: String,
}

/// The condition object for the subscription
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Condition {
    /// The ID of the broadcaster user
    #[serde(skip_serializing_if = "Option::is_none")]
    pub broadcaster_user_id: Option<String>,
}

/// The transport object for the subscription
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Transport {
    /// The callback URL where the notifications are sent. The URL must use the HTTPS protocol
    /// and port 443. See Processing an event. Specify this field only if method is set to
    /// webhook.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub callback: Option<String>,

    /// The UTC date and time that the WebSocket connection was established. This is a
    /// response-only field that Create EventSub Subscription and Get EventSub Subscription
    /// returns if the method field is set to websocket.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub connected_at: Option<String>,

    /// The UTC date and time that the WebSocket connection was lost. This is a response-only
    /// field that Get EventSub Subscription returns if the method field is set to websocket.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub disconnected_at: Option<String>,

    /// The transport method
    pub method: Method,

    /// The secret used to verify the signature. The secret must be an ASCII string that's a
    /// minimum of 10 characters long and a maximum of 100 characters long. For information about
    /// how the secret is used, see Verifying the event message. Specify this field only if
    /// method is set to webhook.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub secret: Option<String>,

    /// An ID that identifies the WebSocket to send notifications to. When you connect to
    /// EventSub using WebSockets, the server returns the ID in the Welcome message. Specify this
    /// field only if method is set to websocket.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub session_id: Option<String>,
}

/// The transport method
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Method {
    Webhook,

    Websocket,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CutList {
    /// Audio channel mixing and volume control configuration
    #[serde(skip_serializing_if = "Option::is_none")]
    pub audio_mixing: Option<Vec<AudioChannelMixing>>,

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

/// Audio mixing configuration for a specific channel
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AudioChannelMixing {
    /// Volume keyframes for this channel throughout the timeline
    #[serde(skip_serializing_if = "Option::is_none")]
    pub keyframes: Option<Vec<AudioChannelKeyframe>>,

    /// 0-indexed output audio channel number
    pub output_channel: i64,

    /// 0-indexed source audio channel number
    pub source_channel: i64,
}

/// A keyframe defining volume level for an audio channel at a specific timeline position
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AudioChannelKeyframe {
    /// Timeline frame position for this keyframe
    pub frame: i64,

    /// Volume level (0.0 = mute, 1.0 = original, >1.0 = amplified)
    pub volume: f64,
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
    pub cut_list: Option<EpisodeCutList>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub error_message: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,

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
    pub series_id: Option<String>,

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
pub struct EpisodeCutList {
    /// Audio channel mixing and volume control configuration
    #[serde(skip_serializing_if = "Option::is_none")]
    pub audio_mixing: Option<Vec<AudioChannelMixing>>,

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

    #[serde(rename = "not_ready_to_upload")]
    NotReadyToUpload,

    #[serde(rename = "ready_to_upload")]
    ReadyToUpload,

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
pub struct Project {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub created_at: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub cut_list: Option<ProjectCutList>,

    /// Optional reference to the episode this project is linked to
    #[serde(skip_serializing_if = "Option::is_none")]
    pub episode_id: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,

    /// Current status of the project - no backend validation enforced
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub updated_at: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub user_id: Option<String>,

    /// Array of video clip IDs that are part of this project
    #[serde(skip_serializing_if = "Option::is_none")]
    pub video_clip_ids: Option<Vec<String>>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectCutList {
    /// Audio channel mixing and volume control configuration
    #[serde(skip_serializing_if = "Option::is_none")]
    pub audio_mixing: Option<Vec<AudioChannelMixing>>,

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

    #[serde(skip_serializing_if = "Option::is_none")]
    pub episode_description_template: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub episode_title_template: Option<String>,

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
pub struct StreamWidget {
    /// Authentication token for WebSocket access to this widget
    #[serde(skip_serializing_if = "Option::is_none")]
    pub access_token: Option<String>,

    /// Whether widget is currently active and should receive scheduled updates
    #[serde(skip_serializing_if = "Option::is_none")]
    pub active: Option<bool>,

    /// Widget configuration settings
    #[serde(skip_serializing_if = "Option::is_none")]
    pub config: Option<HashMap<String, Option<serde_json::Value>>>,

    /// ISO 8601 timestamp when the widget was created
    #[serde(skip_serializing_if = "Option::is_none")]
    pub created_at: Option<String>,

    /// Unique identifier for the stream widget
    pub id: String,

    /// Current widget state data
    #[serde(skip_serializing_if = "Option::is_none")]
    pub state: Option<HashMap<String, Option<serde_json::Value>>>,

    /// Display title for the widget
    pub title: String,

    /// Widget type determines update behavior and available actions
    #[serde(rename = "type")]
    pub stream_widget_type: StreamWidgetType,

    /// ISO 8601 timestamp when the widget was last updated
    #[serde(skip_serializing_if = "Option::is_none")]
    pub updated_at: Option<String>,

    /// The ID of the user who owns this widget
    pub user_id: String,
}

/// Widget type determines update behavior and available actions
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StreamWidgetType {
    #[serde(rename = "bot_integration")]
    BotIntegration,

    Countdown,

    #[serde(rename = "name_queue")]
    NameQueue,

    Poll,

    #[serde(rename = "text_overlay")]
    TextOverlay,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SubscribeChatRequest {
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SubscribeChatResponse {
    /// The status of the subscription request
    pub status: String,

    /// The ID of the created EventSub subscription, if successful
    #[serde(skip_serializing_if = "Option::is_none")]
    pub subscription_id: Option<String>,
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
pub struct TwitchChatMessage {
    pub channel_id: String,

    pub event_type: String,

    pub message: String,

    pub sender_id: String,

    pub timestamp: String,

    pub ttl: i64,

    pub user_id: String,

    pub user_login: String,

    pub user_name: String,
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

    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,

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

/// Base structure for WebSocket messages
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WebSocketMessage {
    /// Type of WebSocket message
    #[serde(rename = "type")]
    pub web_socket_message_type: WebSocketMessageType,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub widget_id: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub action: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub payload: Option<HashMap<String, Option<serde_json::Value>>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub config: Option<HashMap<String, Option<serde_json::Value>>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub state: Option<HashMap<String, Option<serde_json::Value>>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<HashMap<String, Option<serde_json::Value>>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub success: Option<bool>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub task: Option<Task>,
}

/// A task represents a unit of work in the system, with a unique identifier, status,
/// timestamps for creation and updates, type of task, and an associated record ID.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Task {
    pub created_at: String,

    pub id: String,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub record_id: Option<String>,

    pub status: Status,

    pub task_type: TaskType,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub updated_at: Option<String>,

    pub user_id: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum Status {
    Aborted,

    Completed,

    Failed,

    Pending,

    #[serde(rename = "PENDING_REDRIVE")]
    PendingRedrive,

    Running,

    #[serde(rename = "TIMED_OUT")]
    TimedOut,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TaskType {
    Ingestion,

    Rendering,

    Upload,
}

/// Type of WebSocket message
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum WebSocketMessageType {
    #[serde(rename = "TASK_UPDATE")]
    TaskUpdate,

    #[serde(rename = "WIDGET_ACTION")]
    WidgetAction,

    #[serde(rename = "WIDGET_ACTION_RESPONSE")]
    WidgetActionResponse,

    #[serde(rename = "WIDGET_CONFIG_UPDATE")]
    WidgetConfigUpdate,

    #[serde(rename = "WIDGET_STATE_UPDATE")]
    WidgetStateUpdate,

    #[serde(rename = "WIDGET_SUBSCRIBE")]
    WidgetSubscribe,

    #[serde(rename = "WIDGET_UNSUBSCRIBE")]
    WidgetUnsubscribe,
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

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct YouTubeUploadRequest {
    /// Array of episode IDs to upload to YouTube
    pub episode_ids: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct YouTubeUploadResponse {
    /// Status message
    pub message: String,

    /// Number of episodes queued for upload
    #[serde(skip_serializing_if = "Option::is_none")]
    pub queued_count: Option<i64>,

    /// Episodes that failed validation
    #[serde(skip_serializing_if = "Option::is_none")]
    pub validation_errors: Option<Vec<ValidationError>>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ValidationError {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub episode_id: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}
