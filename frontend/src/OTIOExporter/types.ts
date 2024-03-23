export interface Cut {
  start: number;
  end: number;
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
  name: string;
  description: string;
  cuts: CutSequence;
}

export interface VideoClip {
  uri: string;
  duration: number;
}

export interface Stream {
  videoClips: VideoClip[];
}

export interface InternalTrack {
  sourcePath: string;
  sourceStartFrames: number;
  durationFrames: number;

  totalMediaDurationFrames: number;
}
