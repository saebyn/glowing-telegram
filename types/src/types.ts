export interface AccessTokenResponse {
    access_token:   string;
    broadcaster_id: string;
    login:          string;
}

export interface AuthorizationURLResponse {
    url: string;
}

export interface ChatSubscriptionStatusResponse {
    /**
     * Whether the user has any active chat subscriptions
     */
    has_active_subscription: boolean;
    /**
     * Array of active EventSub chat subscriptions for the user
     */
    subscriptions: EventSubSubscription[];
}

export interface EventSubSubscription {
    /**
     * The condition object for the subscription
     */
    condition: Condition;
    /**
     * When the subscription was created
     */
    created_at: string;
    /**
     * The subscription ID
     */
    id: string;
    /**
     * The status of the subscription
     */
    status: string;
    /**
     * The transport object for the subscription
     */
    transport: Transport;
    /**
     * The type of the subscription
     */
    type: string;
    /**
     * The version of the subscription
     */
    version: string;
}

/**
 * The condition object for the subscription
 */
export interface Condition {
    /**
     * The ID of the broadcaster user
     */
    broadcaster_user_id?: string;
    [property: string]: unknown;
}

/**
 * The transport object for the subscription
 */
export interface Transport {
    /**
     * The callback URL where the notifications are sent. The URL must use the HTTPS protocol
     * and port 443. See Processing an event. Specify this field only if method is set to
     * webhook.
     */
    callback?: string;
    /**
     * The UTC date and time that the WebSocket connection was established. This is a
     * response-only field that Create EventSub Subscription and Get EventSub Subscription
     * returns if the method field is set to websocket.
     */
    connected_at?: string;
    /**
     * The UTC date and time that the WebSocket connection was lost. This is a response-only
     * field that Get EventSub Subscription returns if the method field is set to websocket.
     */
    disconnected_at?: string;
    /**
     * The transport method
     */
    method: Method;
    /**
     * The secret used to verify the signature. The secret must be an ASCII string that's a
     * minimum of 10 characters long and a maximum of 100 characters long. For information about
     * how the secret is used, see Verifying the event message. Specify this field only if
     * method is set to webhook.
     */
    secret?: string;
    /**
     * An ID that identifies the WebSocket to send notifications to. When you connect to
     * EventSub using WebSockets, the server returns the ID in the Welcome message. Specify this
     * field only if method is set to websocket.
     */
    session_id?: string;
    [property: string]: unknown;
}

/**
 * The transport method
 */
export type Method = "webhook" | "websocket";

export interface CutList {
    /**
     * Audio channel mixing and volume control configuration
     */
    audioMixing?: AudioChannelMixing[];
    /**
     * List of input media sources
     */
    inputMedia: InputMedia[];
    /**
     * Ordered media sections to form the output timeline sequence
     */
    outputTrack: OutputTrack[];
    /**
     * One or more overlay tracks
     */
    overlayTracks?: OverlayTrack[];
    /**
     * Schema version
     */
    version: "1.0.0";
}

/**
 * Audio mixing configuration for a specific channel
 */
export interface AudioChannelMixing {
    /**
     * Volume keyframes for this channel throughout the timeline
     */
    keyframes?: AudioChannelKeyframe[];
    /**
     * 0-indexed output audio channel number
     */
    outputChannel: number;
    /**
     * 0-indexed source audio channel number
     */
    sourceChannel: number;
    [property: string]: unknown;
}

/**
 * A keyframe defining volume level for an audio channel at a specific timeline position
 */
export interface AudioChannelKeyframe {
    /**
     * Timeline frame position for this keyframe
     */
    frame: number;
    /**
     * Volume level (0.0 = mute, 1.0 = original, >1.0 = amplified)
     */
    volume: number;
    [property: string]: unknown;
}

export interface InputMedia {
    /**
     * Path of the media
     */
    s3Location: string;
    /**
     * Start/end frames to select
     */
    sections: MediaSection[];
}

export interface MediaSection {
    /**
     * End frame is exclusive
     */
    endFrame:   number;
    startFrame: number;
}

export interface OutputTrack {
    /**
     * Index of the media source
     */
    mediaIndex: number;
    /**
     * Index of the section in the media source
     */
    sectionIndex: number;
    /**
     * Transition to apply at the start of the section
     */
    transitionIn?: TransitionInObject;
    /**
     * Transition to apply at the end of the section
     */
    transitionOut?: TransitionOutObject;
}

/**
 * Transition to apply at the start of the section
 *
 * Transition to apply at the start or end of a media section
 */
export interface TransitionInObject {
    /**
     * Duration of the transition in frames, relative to the start/end of the section
     */
    duration: number;
    /**
     * Transition type
     */
    type: TransitionInType;
    [property: string]: unknown;
}

/**
 * Transition type
 */
export type TransitionInType = "fade" | "cut";

/**
 * Transition to apply at the end of the section
 *
 * Transition to apply at the start of the section
 *
 * Transition to apply at the start or end of a media section
 */
export interface TransitionOutObject {
    /**
     * Duration of the transition in frames, relative to the start/end of the section
     */
    duration: number;
    /**
     * Transition type
     */
    type: TransitionInType;
    [property: string]: unknown;
}

export interface OverlayTrack {
    /**
     * Index of the media source
     */
    mediaIndex: number;
    /**
     * Index of the section in the media source
     */
    sectionIndex: number;
    /**
     * Start frame on the overlay track
     */
    startFrame: number;
    /**
     * Overlay type
     */
    type: OverlayTrackType;
    /**
     * X position of the overlay
     */
    x?: number;
    /**
     * Y position of the overlay
     */
    y?: number;
}

/**
 * Overlay type
 */
export type OverlayTrackType = "alpha" | "colorkey";

export interface Episode {
    category?:              number;
    created_at?:            string;
    cut_list?:              CutListClass;
    description?:           string;
    error_message?:         string;
    id?:                    string;
    is_published?:          boolean;
    notify_subscribers?:    boolean;
    order_index?:           number;
    render_uri?:            string;
    retry_after_seconds?:   number;
    series_id?:             string;
    stream_id?:             string;
    tags?:                  string[];
    title?:                 string;
    tracks?:                Track[];
    updated_at?:            string;
    upload_attempts?:       number;
    upload_resume_at_byte?: number;
    upload_status?:         UploadStatus;
    user_id?:               string;
    youtube_upload_url?:    string;
    youtube_video_id?:      string;
}

export interface CutListClass {
    /**
     * Audio channel mixing and volume control configuration
     */
    audioMixing?: AudioChannelMixing[];
    /**
     * List of input media sources
     */
    inputMedia: InputMedia[];
    /**
     * Ordered media sections to form the output timeline sequence
     */
    outputTrack: OutputTrack[];
    /**
     * One or more overlay tracks
     */
    overlayTracks?: OverlayTrack[];
    /**
     * Schema version
     */
    version: "1.0.0";
}

export interface Track {
    end:   string;
    start: string;
}

export type UploadStatus = "FAILED" | "SUCCESS" | "THROTTLED" | "ready_to_upload" | "not_ready_to_upload";

export interface IDOnly {
    id: string;
}

export interface Profile {
    id: string;
}

/**
 * Represents a project that combines cuts from multiple streams for episode creation
 */
export interface Project {
    created_at?: string;
    /**
     * List of cuts included in the project, with timing and source information
     */
    cuts?: CutElement[];
    /**
     * Optional reference to the episode this project is linked to
     */
    episode_id?: string;
    id?:         string;
    /**
     * Current status of the project - no backend validation enforced
     */
    status?:     string;
    title?:      string;
    updated_at?: string;
    user_id?:    string;
}

/**
 * A clip representing a cut from a source stream, with start and end
 */
export interface CutElement {
    /**
     * End time of the cut in seconds (relative to the start of the stream)
     */
    end_time: number;
    /**
     * Start time of the cut in seconds (relative to the start of the stream)
     */
    start_time: number;
    /**
     * ID of the source stream for this cut
     */
    stream_id: string;
    /**
     * Title or description for this cut
     */
    title: string;
    [property: string]: unknown;
}

export interface RenderRequest {
    episodeIds: string[];
}

export interface Series {
    category?:                     number;
    created_at:                    string;
    description?:                  string;
    end_date?:                     string;
    end_time?:                     string;
    episode_description_template?: string;
    episode_title_template?:       string;
    id:                            string;
    is_active?:                    boolean;
    max_episode_order_index?:      number;
    notify_subscribers?:           boolean;
    playlist_id?:                  string;
    prep_notes?:                   string;
    recurrence?:                   Recurrence;
    skips?:                        Skip[];
    start_date?:                   string;
    start_time?:                   string;
    stream_count?:                 number;
    stream_title_template?:        string;
    tags?:                         string[];
    thumbnail_url?:                string;
    timezone?:                     string;
    title:                         string;
    twitch_category?:              TwitchCategory;
    updated_at?:                   string;
}

export interface Recurrence {
    days:     Day[];
    interval: number;
    type:     "weekly";
}

export type Day = "sunday" | "monday" | "tuesday" | "wednesday" | "thursday" | "friday" | "saturday";

export interface Skip {
    date:   string;
    reason: string;
}

export interface TwitchCategory {
    box_art_url?: string;
    id:           string;
    name:         string;
}

export interface SimpleChatMessage {
    content: string;
    role:    Role;
}

export type Role = "system" | "user" | "assistant" | "function" | "tool";

export interface Stream {
    created_at?:       string;
    description?:      string;
    duration?:         number;
    has_episodes?:     boolean;
    id:                string;
    prefix?:           string;
    series_id?:        string;
    stream_date?:      string;
    stream_platform?:  string;
    thumbnail_url?:    string;
    title?:            string;
    updated_at?:       string;
    video_clip_count?: number;
}

/**
 * A clip representing a cut from a source stream, with start and end
 */
export interface StreamClip {
    /**
     * End time of the cut in seconds (relative to the start of the stream)
     */
    end_time: number;
    /**
     * Start time of the cut in seconds (relative to the start of the stream)
     */
    start_time: number;
    /**
     * ID of the source stream for this cut
     */
    stream_id: string;
    /**
     * Title or description for this cut
     */
    title: string;
    [property: string]: unknown;
}

export interface StreamIngestionRequest {
    initialPrompt:  string;
    initialSummary: string;
    streamId:       string;
}

export interface StreamWidget {
    /**
     * Authentication token for WebSocket access to this widget
     */
    access_token?: string;
    /**
     * Whether widget is currently active and should receive scheduled updates
     */
    active?: boolean;
    /**
     * Widget configuration settings
     */
    config?: { [key: string]: any };
    /**
     * ISO 8601 timestamp when the widget was created
     */
    created_at?: string;
    /**
     * Unique identifier for the stream widget
     */
    id: string;
    /**
     * Current widget state data
     */
    state?: { [key: string]: any };
    /**
     * Display title for the widget
     */
    title: string;
    /**
     * Widget type determines update behavior and available actions
     */
    type: StreamWidgetType;
    /**
     * ISO 8601 timestamp when the widget was last updated
     */
    updated_at?: string;
    /**
     * The ID of the user who owns this widget
     */
    user_id: string;
}

/**
 * Widget type determines update behavior and available actions
 */
export type StreamWidgetType = "countdown" | "text_overlay" | "poll" | "name_queue" | "bot_integration";

export interface SubscribeChatRequest {
}

export interface SubscribeChatResponse {
    /**
     * The status of the subscription request
     */
    status: string;
    /**
     * The ID of the created EventSub subscription, if successful
     */
    subscription_id?: null | string;
}

export interface TwitchAuthRequest {
    redirect_uri: string;
    scopes:       string[];
}

export interface TwitchCallbackRequest {
    code:  string;
    scope: string[];
    state: string;
}

export interface TwitchCallbackResponse {
    /**
     * The URL to redirect the client to after the authorization flow is complete.
     */
    url: string;
}

export interface TwitchChatMessage {
    channel_id: string;
    event_type: string;
    message:    string;
    sender_id:  string;
    timestamp:  string;
    ttl:        number;
    user_id:    string;
    user_login: string;
    user_name:  string;
}

export interface TwitchSessionSecret {
    access_token?:  string;
    csrf_token:     string;
    redirect_url:   string;
    refresh_token?: string;
    scopes:         string[];
    valid_until?:   number;
}

export interface VideoClip {
    /**
     * The path to the audio file extracted from the video clip.
     */
    audio?: string;
    id?:    string;
    /**
     * The S3 key of the video clip.
     */
    key: string;
    /**
     * A list of paths to images that are keyframes in the video clip.
     */
    keyframes?: string[];
    metadata?:  Metadata;
    /**
     * The list of detected silence intervals in the video clip.
     */
    silence?: Silence[];
    /**
     * The start time of the video clip in the context of the stream in seconds.
     */
    start_time?:    number;
    stream_id?:     string;
    summary?:       Summary;
    transcription?: Transcription;
}

export interface Metadata {
    format?: Format;
    [property: string]: unknown;
}

export interface Format {
    /**
     * The duration of the video clip in seconds.
     */
    duration?: number;
    [property: string]: unknown;
}

export interface Silence {
    end?:   number;
    start?: number;
    [property: string]: unknown;
}

export interface Summary {
    attentions?:              Attention[];
    highlights?:              Highlight[];
    keywords?:                string[];
    summary_context?:         string;
    summary_main_discussion?: string;
    title?:                   string;
    transcription_errors?:    TranscriptionError[];
    [property: string]: unknown;
}

export interface Attention {
    description?:     string;
    reasoning?:       string;
    timestamp_end?:   number;
    timestamp_start?: number;
    [property: string]: unknown;
}

export interface Highlight {
    description?:     string;
    reasoning?:       string;
    timestamp_end?:   number;
    timestamp_start?: number;
    [property: string]: unknown;
}

export interface TranscriptionError {
    description?:     string;
    reasoning?:       string;
    timestamp_start?: number;
    [property: string]: unknown;
}

export interface Transcription {
    language: string;
    segments: TranscriptSegment[];
    text:     string;
}

export interface TranscriptSegment {
    avg_logprob:       number;
    compression_ratio: number;
    end:               number;
    no_speech_prob:    number;
    start:             number;
    temperature:       number;
    text:              string;
    tokens:            number[];
}

/**
 * Base structure for WebSocket messages
 */
export interface WebSocketMessage {
    /**
     * Type of WebSocket message
     */
    type:      WebSocketMessageType;
    widgetId?: string;
    action?:   string;
    payload?:  { [key: string]: any };
    config?:   { [key: string]: any };
    state?:    { [key: string]: any };
    error?:    string;
    result?:   { [key: string]: any };
    success?:  boolean;
    task?:     Task;
    [property: string]: unknown;
}

/**
 * A task represents a unit of work in the system, with a unique identifier, status,
 * timestamps for creation and updates, type of task, and an associated record ID.
 */
export interface Task {
    created_at:  string;
    id:          string;
    record_id?:  string;
    status:      Status;
    task_type:   TaskType;
    updated_at?: string;
    user_id:     string;
}

export type Status = "PENDING" | "RUNNING" | "COMPLETED" | "FAILED" | "TIMED_OUT" | "ABORTED" | "PENDING_REDRIVE";

export type TaskType = "ingestion" | "upload" | "rendering";

/**
 * Type of WebSocket message
 */
export type WebSocketMessageType = "WIDGET_SUBSCRIBE" | "WIDGET_UNSUBSCRIBE" | "WIDGET_ACTION" | "WIDGET_CONFIG_UPDATE" | "WIDGET_STATE_UPDATE" | "WIDGET_ACTION_RESPONSE" | "TASK_UPDATE";

export interface YouTubeAuthRequest {
    redirect_uri: string;
    scopes:       string[];
}

export interface YouTubeCallbackRequest {
    code:  string;
    scope: string[];
    state: string;
}

export interface YouTubeCallbackResponse {
    /**
     * The URL to redirect the client to after the authorization flow is complete.
     */
    url: string;
}

export interface YouTubeSessionSecret {
    access_token?:  string;
    csrf_token:     string;
    redirect_url:   string;
    refresh_token?: string;
    scopes:         string[];
    valid_until?:   number;
}

export interface YouTubeUploadRequest {
    /**
     * Array of episode IDs to upload to YouTube
     */
    episode_ids: string[];
}

export interface YouTubeUploadResponse {
    /**
     * Status message
     */
    message: string;
    /**
     * Number of episodes queued for upload
     */
    queued_count?: number;
    /**
     * Episodes that failed validation
     */
    validation_errors?: ValidationError[];
}

export interface ValidationError {
    episode_id?: string;
    error?:      string;
    [property: string]: unknown;
}
