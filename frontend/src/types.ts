export interface TranscriptSegment {
  start: string;
  end: string;
  text: string;
}

export interface Cut {
  start: string;
  end: string;
}

/**
 * A sequence of cuts that make up an episode.
 *
 * The cuts are in order, and the end of one cut is the start of the next.
 * The value of each `start` and `end` is relative to the start of the
 * overall stream of media that the media is being cut from.
 */
export type CutSequence = Cut[];

export interface Episode {
  id?: string;
  stream_id?: string;
  title: string;
  description: string;
  tracks: CutSequence;
}

export interface VideoClip {
  uri: string;
  duration: number;
}

export interface Stream {
  transcription_segments?: TranscriptSegment[];

  video_clips: VideoClip[];
}

export interface YoutubeUploadTaskPayload {
  title: string;
  description: string;
  tags: string[];
  category: number;
  render_uri: string;
  notify_subscribers: boolean;
}
