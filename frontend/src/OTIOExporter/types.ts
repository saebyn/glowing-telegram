export interface InternalTrack {
  sourcePath: string;
  sourceStartFrames: number;
  durationFrames: number;

  totalMediaDurationFrames: number;
}

export interface ConvertedCut {
  start: number;
  end: number;
}

export interface ConvertedEpisode {
  title: string;
  description: string;
  tracks: ConvertedCut[];
}
