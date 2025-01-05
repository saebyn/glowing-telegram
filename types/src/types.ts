export interface AccessTokenResponse {
    access_token:   string;
    broadcaster_id: string;
}

export interface AuthorizationURLResponse {
    url: string;
}

export interface Episode {
    description?: string;
    id:           string;
    stream_id?:   string;
    title?:       string;
    tracks?:      Track[];
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
