import { expect, describe, it } from "vitest";

import {
  findMediaClipCursorStart,
  findMediaClipCursorEnd,
  findMediaClipCursors,
} from "./mediaClipSequence";
import { CutSequence } from "./types";

describe("mediaClipSequence", () => {
  describe("findMediaClipCursorStart", () => {
    it("should return the index of the media clip that contains the cursor", () => {
      const mediaClips: CutSequence = [
        { start: 0, end: 100 },
        { start: 200, end: 300 },
      ];

      expect(findMediaClipCursorStart(mediaClips, 50)).toMatchObject({
        clipIndex: 0,
        duration: 50,
        time: 50,
      });
      expect(findMediaClipCursorStart(mediaClips, 250)).toMatchObject({
        clipIndex: 1,
        duration: 50,
      });
      expect(findMediaClipCursorStart(mediaClips, 450)).toBe(null);
    });
  });

  describe("findMediaClipCursorEnd", () => {
    it("should return the index of the media clip that contains the cursor", () => {
      const mediaClips: CutSequence = [
        { start: 0, end: 100 },
        { start: 200, end: 300 },
      ];

      expect(findMediaClipCursorEnd(mediaClips, 300)).toMatchObject({
        clipIndex: 1,
        duration: 0,
        time: 0,
      });
    });

    it("should find the cursor for the last clip", () => {
      const mediaClips: CutSequence = [
        { start: 0, end: 100 },
        { start: 100, end: 300 },
        { start: 300, end: 400 },
      ];

      expect(findMediaClipCursorEnd(mediaClips, 400)).toMatchObject({
        clipIndex: 2,
        duration: 0,
        time: 0,
      });
    });
  });

  describe("findMediaClipCursors", () => {
    it("should return the start and end cursors for a given time", () => {
      const mediaClips: CutSequence = [
        { start: 0, end: 100 },
        { start: 100, end: 300 },
        { start: 300, end: 400 },
      ];

      const start = findMediaClipCursorStart(mediaClips, 50);
      const end = findMediaClipCursorEnd(mediaClips, 350);

      expect(start).not.toBe(null);
      expect(end).not.toBe(null);

      expect(findMediaClipCursors(mediaClips, start!, end!)).toMatchObject([
        { clipIndex: 1, duration: 50, time: 0 },
      ]);
    });
  });
});
