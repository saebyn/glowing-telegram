import { FPS } from "./constants";
import { CutSequence, InternalTrack, VideoClip } from "./types";

interface MediaClipCursor {
  clipIndex: number;
  time: number;
  duration: number;
}

export function findMediaClipCursorStart(
  clips: CutSequence,
  time: number
): MediaClipCursor | null {
  const clip = clips.find((clip) => {
    if (time >= clip.start && time < clip.end) {
      return true;
    }
  });

  if (clip) {
    return {
      clipIndex: clips.indexOf(clip),
      time: time - clip.start,
      duration: Math.min(clip.end - time, clip.end - clip.start),
    };
  }

  return null;
}

export function findMediaClipCursorEnd(
  clips: CutSequence,
  time: number
): MediaClipCursor | null {
  const clip = clips.find((clip) => {
    if (time > clip.start && time <= clip.end) {
      return true;
    }
  });

  if (clip) {
    return {
      clipIndex: clips.indexOf(clip),
      time: 0,
      duration: clip.end - time,
    };
  }

  return null;
}

export function findMediaClipCursors(
  _clips: CutSequence,
  start: MediaClipCursor,
  end: MediaClipCursor
): MediaClipCursor[] {
  const cursors = [];

  for (let i = start.clipIndex + 1; i < end.clipIndex; i++) {
    cursors.push({
      clipIndex: i,
      time: 0,
      duration: end.duration,
    });
  }

  return cursors;
}

export function sameMediaClip(a: MediaClipCursor, b: MediaClipCursor): boolean {
  return a.clipIndex === b.clipIndex;
}

export function convertMediaClipCursorToInternalTrack(
  videoClips: VideoClip[],
  cursor: MediaClipCursor
): InternalTrack {
  const clip = videoClips[cursor.clipIndex];
  return {
    sourcePath: clip.uri,
    sourceStartFrames: cursor.time * FPS,
    durationFrames: cursor.duration * FPS,
    totalMediaDurationFrames: clip.duration * FPS,
  };
}
