export interface AccessTokenResponse {
    access_token:   string;
    broadcaster_id: string;
}

export interface AuthorizationURLResponse {
    url: string;
}

export interface CutList {
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
     * X position of the overlay
     */
    x?: number;
    /**
     * Y position of the overlay
     */
    y?: number;
}

export interface Episode {
    cut_list?:    CutListClass;
    description?: string;
    id:           string;
    order_index?: number;
    stream_id?:   string;
    title?:       string;
    tracks?:      Track[];
}

export interface CutListClass {
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

export interface IDOnly {
    id: string;
}

export interface Profile {
    id: string;
}

export interface RenderRequest {
    episodeIds: string[];
}

export interface Series {
    category?:                number;
    created_at:               string;
    description?:             string;
    end_date?:                string;
    end_time?:                string;
    id:                       string;
    is_active?:               boolean;
    max_episode_order_index?: number;
    notify_subscribers?:      boolean;
    playlist_id?:             string;
    prep_notes?:              string;
    recurrence?:              Recurrence;
    skips?:                   Skip[];
    start_date?:              string;
    start_time?:              string;
    stream_count?:            number;
    stream_title_template?:   string;
    tags?:                    string[];
    thumbnail_url?:           string;
    timezone?:                string;
    title:                    string;
    twitch_category?:         TwitchCategory;
    updated_at?:              string;
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

export interface StreamIngestionRequest {
    initialPrompt:  string;
    initialSummary: string;
    streamId:       string;
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
    id:     string;
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
