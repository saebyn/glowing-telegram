export interface AccessTokenResponse {
    accessToken: string;
}

export interface AuthorizationURLResponse {
    url: string;
}

export interface Episode {
    description?: string;
    id:           string;
    streamID?:    string;
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

export interface SimpleChatMessage {
    content: string;
    role:    Role;
}

export type Role = "system" | "user" | "assistant" | "function" | "tool";

export interface Stream {
    createdAt?:      Date;
    description?:    string;
    duration?:       number;
    hasEpisodes?:    boolean;
    id:              string;
    prefix?:         string;
    seriesID?:       string;
    streamDate?:     Date;
    streamPlatform?: string;
    thumbnailURL?:   string;
    title?:          string;
    updatedAt?:      Date;
    videoClipCount?: number;
}

export interface StreamIngestionRequest {
    initialPrompt:  string;
    initialSummary: string;
    streamID:       string;
}

export interface TwitchAuthRequest {
    redirectURI: string;
}

export interface TwitchCallbackRequest {
    code:  string;
    scope: string[];
    state: string;
}

export interface VideoClip {
    audio?:         string;
    key?:           string;
    keyframes?:     string[];
    metadata?:      Metadata;
    silence?:       Silence[];
    startTime?:     number;
    streamID?:      string;
    summary?:       Summary;
    transcription?: Transcription;
    [property: string]: unknown;
}

export interface Metadata {
    format?: Format;
    [property: string]: unknown;
}

export interface Format {
    duration?: number;
    [property: string]: unknown;
}

export interface Silence {
    end?:   number;
    start?: number;
    [property: string]: unknown;
}

export interface Summary {
    attentions?:            Attention[];
    highlights?:            Highlight[];
    keywords?:              string[];
    summaryContext?:        string;
    summaryMainDiscussion?: string;
    title?:                 string;
    transcriptionErrors?:   TranscriptionError[];
    [property: string]: unknown;
}

export interface Attention {
    description?:    number;
    reasoning?:      number;
    timestampEnd?:   number;
    timestampStart?: number;
    [property: string]: unknown;
}

export interface Highlight {
    description?:    number;
    reasoning?:      number;
    timestampEnd?:   number;
    timestampStart?: number;
    [property: string]: unknown;
}

export interface TranscriptionError {
    description?:    number;
    reasoning?:      number;
    timestampStart?: number;
    [property: string]: unknown;
}

export interface Transcription {
    language?: string;
    segments?: SegmentElement[];
    text?:     string;
    [property: string]: unknown;
}

export interface SegmentElement {
    avgLogprob?:       number;
    compressionRatio?: number;
    end?:              number;
    noSpeechProb?:     number;
    start?:            number;
    temperature?:      number;
    text?:             string;
    [property: string]: unknown;
}
