import { parseIntoSeconds } from "../isoDuration";
import { Episode, Stream, VideoClip } from "../types";
import { FPS } from "./constants";
import floatJsonSerializer, { Float } from "./floatJson";
import {
  findMediaClipCursorStart,
  findMediaClipCursorEnd,
  sameMediaClip,
  findMediaClipCursors,
  convertMediaClipCursorToInternalTrack,
} from "./mediaClipSequence";
import { ConvertedCut, ConvertedEpisode, InternalTrack } from "./types";

/**
 * Extracts the filename from a path.
 * Should return the last part of the path, after the last '/' or '\'.
 *
 * @param path The path to the file to extract the name from.
 */
function filename(path: string): string {
  const parts = path.split(/[\\/]/);
  return parts[parts.length - 1];
}

function internalToOTIO(children: InternalTrack[]): string {
  const startTransition = {
    OTIO_SCHEMA: "Transition.1",
    metadata: {
      Resolve_OTIO: {
        Effects: {
          "Effect Name": "Hexagon Iris",
          Enabled: true,
          Name: "Hexagon Iris",
          Parameters: [
            {
              "Default Parameter Value": new Float(0),
              "Key Frames": {
                "0": {
                  Value: new Float(0),
                  "Variant Type": "Double",
                },
                "20": {
                  Value: new Float(1),
                  "Variant Type": "Double",
                },
              },
              "Parameter ID": "transitionCustomCurvesKeyframes",
              "Parameter Value": new Float(0.0),
              "Variant Type": "Double",
              maxValue: new Float(1),
              minValue: new Float(0),
            },
          ],
          Type: 46,
        },
        "Transition Type": "Hexagon Iris",
      },
    },
    name: "Hexagon Iris",
    in_offset: {
      OTIO_SCHEMA: "RationalTime.1",
      rate: new Float(60),
      value: new Float(0),
    },
    out_offset: {
      OTIO_SCHEMA: "RationalTime.1",
      rate: new Float(60),
      value: new Float(20.0),
    },
    transition_type: "Custom_Transition",
  };

  const videoSubTracks = children.map((track) => {
    // find the video clip that contains the start of the track

    return {
      OTIO_SCHEMA: "Clip.2",
      metadata: {
        Resolve_OTIO: {},
      },
      name: filename(track.sourcePath),
      source_range: {
        OTIO_SCHEMA: "TimeRange.1",
        duration: {
          OTIO_SCHEMA: "RationalTime.1",
          rate: new Float(60),
          value: new Float(track.durationFrames),
        },
        start_time: {
          OTIO_SCHEMA: "RationalTime.1",
          rate: new Float(60),
          value: new Float(track.sourceStartFrames),
        },
      },
      effects: [],
      markers: [],
      enabled: true,
      media_references: {
        DEFAULT_MEDIA: {
          OTIO_SCHEMA: "ExternalReference.1",
          metadata: {},
          name: filename(track.sourcePath),
          available_range: {
            OTIO_SCHEMA: "TimeRange.1",
            duration: {
              OTIO_SCHEMA: "RationalTime.1",
              rate: new Float(60),
              value: new Float(Math.round(track.totalMediaDurationFrames)),
            },
            start_time: {
              OTIO_SCHEMA: "RationalTime.1",
              rate: new Float(60),
              value: new Float(0),
            },
          },
          available_image_bounds: null,
          target_url: track.sourcePath,
        },
      },
      active_media_reference_key: "DEFAULT_MEDIA",
    };
  });

  const videoTrack = {
    OTIO_SCHEMA: "Track.1",
    metadata: {
      Resolve_OTIO: {
        Locked: false,
      },
    },
    name: "Video 1",
    source_range: null,
    effects: [],
    markers: [],
    enabled: true,
    children: [startTransition, ...videoSubTracks],
    kind: "Video",
  };

  const audioTracks = [1, 2, 3].map((sourceTrackId) => ({
    OTIO_SCHEMA: "Track.1",
    metadata: {
      Resolve_OTIO: {
        "Audio Type": "Stereo",
        Locked: false,
        SoloOn: false,
      },
    },
    name: `Audio ${sourceTrackId}`,
    source_range: null,
    effects: [],
    markers: [],
    enabled: true,
    children: videoSubTracks.map((track) => {
      return {
        OTIO_SCHEMA: "Clip.2",
        metadata: {
          Resolve_OTIO: {
            Channels: [
              {
                "Source Channel ID": 0,
                "Source Track ID": sourceTrackId,
              },
              {
                "Source Channel ID": 1,
                "Source Track ID": sourceTrackId,
              },
            ],
          },
        },
        name: track.name,
        source_range: track.source_range,
        effects: [],
        markers: [],
        enabled: true,
        media_references: track.media_references,
        active_media_reference_key: "DEFAULT_MEDIA",
      };
    }),
    kind: "Audio",
  }));

  const overlayTrack = {
    OTIO_SCHEMA: "Track.1",
    metadata: {
      Resolve_OTIO: {
        Locked: false,
      },
    },
    name: "Video 2",
    source_range: null,
    effects: [],
    markers: [],
    enabled: true,
    children: [
      {
        OTIO_SCHEMA: "Clip.2",
        metadata: {
          Resolve_OTIO: {},
        },
        name: "Solid Color",
        source_range: {
          OTIO_SCHEMA: "TimeRange.1",
          duration: {
            OTIO_SCHEMA: "RationalTime.1",
            rate: new Float(60),
            value: new Float(1910),
          },
          start_time: {
            OTIO_SCHEMA: "RationalTime.1",
            rate: new Float(60),
            value: new Float(0),
          },
        },
        effects: [
          {
            OTIO_SCHEMA: "Effect.1",
            metadata: {
              Resolve_OTIO: {
                "Effect Name": "Transform",
                Enabled: true,
                Name: "Transform",
                Parameters: [],
                Type: 2,
              },
            },
            name: "",
            effect_name: "Resolve Effect",
          },
          {
            OTIO_SCHEMA: "Effect.1",
            metadata: {
              Resolve_OTIO: {
                "Effect Name": "Cropping",
                Enabled: true,
                Name: "Cropping",
                Parameters: [],
                Type: 3,
              },
            },
            name: "",
            effect_name: "Resolve Effect",
          },
          {
            OTIO_SCHEMA: "Effect.1",
            metadata: {
              Resolve_OTIO: {
                "Effect Name": "Composite",
                Enabled: true,
                Name: "Composite",
                Parameters: [
                  {
                    "Default Parameter Value": new Float(100),
                    "Key Frames": {},
                    "Parameter ID": "opacity",
                    "Parameter Value": new Float(0.0),
                    "Variant Type": "Double",
                    maxValue: new Float(100),
                    minValue: new Float(0),
                  },
                ],
                Type: 1,
              },
            },
            name: "",
            effect_name: "Resolve Effect",
          },
          {
            OTIO_SCHEMA: "Effect.1",
            metadata: {
              Resolve_OTIO: {
                "Effect Name": "Video Faders",
                Enabled: true,
                Name: "Video Faders",
                Parameters: [],
                Type: 36,
              },
            },
            name: "",
            effect_name: "Resolve Effect",
          },
        ],
        markers: [],
        enabled: true,
        media_references: {
          DEFAULT_MEDIA: {
            OTIO_SCHEMA: "GeneratorReference.1",
            metadata: {
              Resolve_OTIO: {
                "Generator Type": "Solid Color",
              },
            },
            name: "Solid Color",
            available_range: null,
            available_image_bounds: null,
            generator_kind: "Solid Color",
            parameters: {
              Resolve_OTIO: [
                {
                  "Effect Name": "Solid Color",
                  Enabled: true,
                  Name: "Generator",
                  Parameters: [
                    {
                      "Default Parameter Value": "",
                      "Parameter ID": "Display Name",
                      "Parameter Value": "",
                      "Variant Type": "String",
                    },
                    {
                      "Default Parameter Value": "#000000",
                      "Parameter ID": "color",
                      "Parameter Value": "#000000",
                      "Variant Type": "Color",
                    },
                  ],
                  Type: 5,
                },
              ],
            },
          },
        },
        active_media_reference_key: "DEFAULT_MEDIA",
      },
      {
        OTIO_SCHEMA: "Clip.2",
        metadata: {
          Resolve_OTIO: {},
        },
        name: "LiveOnTwitch",
        source_range: {
          OTIO_SCHEMA: "TimeRange.1",
          duration: {
            OTIO_SCHEMA: "RationalTime.1",
            rate: new Float(60),
            value: new Float(106),
          },
          start_time: {
            OTIO_SCHEMA: "RationalTime.1",
            rate: new Float(60),
            value: new Float(0),
          },
        },
        effects: [],
        markers: [],
        enabled: true,
        media_references: {
          DEFAULT_MEDIA: {
            OTIO_SCHEMA: "MissingReference.1",
            metadata: {},
            name: "",
            available_range: null,
            available_image_bounds: null,
          },
        },
        active_media_reference_key: "DEFAULT_MEDIA",
      },
      {
        OTIO_SCHEMA: "Gap.1",
        metadata: {},
        name: "",
        source_range: {
          OTIO_SCHEMA: "TimeRange.1",
          duration: {
            OTIO_SCHEMA: "RationalTime.1",
            rate: new Float(60),
            value: new Float(1816),
          },
          start_time: {
            OTIO_SCHEMA: "RationalTime.1",
            rate: new Float(60),
            value: new Float(0),
          },
        },
        effects: [],
        markers: [],
        enabled: true,
      },
      {
        OTIO_SCHEMA: "Clip.2",
        metadata: {
          Resolve_OTIO: {},
        },
        name: "LikeReminder1",
        source_range: {
          OTIO_SCHEMA: "TimeRange.1",
          duration: {
            OTIO_SCHEMA: "RationalTime.1",
            rate: new Float(60),
            value: new Float(300),
          },
          start_time: {
            OTIO_SCHEMA: "RationalTime.1",
            rate: new Float(60),
            value: new Float(0),
          },
        },
        effects: [],
        markers: [],
        enabled: true,
        media_references: {
          DEFAULT_MEDIA: {
            OTIO_SCHEMA: "MissingReference.1",
            metadata: {},
            name: "",
            available_range: null,
            available_image_bounds: null,
          },
        },
        active_media_reference_key: "DEFAULT_MEDIA",
      },
      {
        OTIO_SCHEMA: "Gap.1",
        metadata: {},
        name: "",
        source_range: {
          OTIO_SCHEMA: "TimeRange.1",
          duration: {
            OTIO_SCHEMA: "RationalTime.1",
            rate: new Float(60),
            value: new Float(186387.0),
          },
          start_time: {
            OTIO_SCHEMA: "RationalTime.1",
            rate: new Float(60),
            value: new Float(0),
          },
        },
        effects: [],
        markers: [],
        enabled: true,
      },
      {
        OTIO_SCHEMA: "Transition.1",
        metadata: {
          Resolve_OTIO: {
            Effects: {
              "Effect Name": "Cross Dissolve",
              Enabled: true,
              Name: "Cross Dissolve",
              Parameters: [
                {
                  "Default Parameter Value": new Float(0),
                  "Key Frames": {
                    "0": {
                      Value: new Float(0),
                      "Variant Type": "Double",
                    },
                    "194": {
                      Value: new Float(1),
                      "Variant Type": "Double",
                    },
                  },
                  "Parameter ID": "transitionCustomCurvesKeyframes",
                  "Parameter Value": new Float(0),
                  "Variant Type": "Double",
                  maxValue: new Float(1),
                  minValue: new Float(0),
                },
              ],
              Type: 9,
            },
            "Transition Type": "Cross Dissolve",
          },
        },
        name: "Cross Dissolve",
        in_offset: {
          OTIO_SCHEMA: "RationalTime.1",
          rate: new Float(60),
          value: new Float(0),
        },
        out_offset: {
          OTIO_SCHEMA: "RationalTime.1",
          rate: new Float(60),
          value: new Float(194.0),
        },
        transition_type: "SMPTE_Dissolve",
      },
      {
        OTIO_SCHEMA: "Clip.2",
        metadata: {
          Resolve_OTIO: {},
        },
        name: "outro.mov",
        source_range: {
          OTIO_SCHEMA: "TimeRange.1",
          duration: {
            OTIO_SCHEMA: "RationalTime.1",
            rate: new Float(60),
            value: new Float(1400.0),
          },
          start_time: {
            OTIO_SCHEMA: "RationalTime.1",
            rate: new Float(60),
            value: new Float(400),
          },
        },
        effects: [],
        markers: [],
        enabled: true,
        media_references: {
          DEFAULT_MEDIA: {
            OTIO_SCHEMA: "ExternalReference.1",
            metadata: {},
            name: "outro.mov",
            available_range: {
              OTIO_SCHEMA: "TimeRange.1",
              duration: {
                OTIO_SCHEMA: "RationalTime.1",
                rate: new Float(60),
                value: new Float(1800.0),
              },
              start_time: {
                OTIO_SCHEMA: "RationalTime.1",
                rate: new Float(60),
                value: new Float(0),
              },
            },
            available_image_bounds: null,
            target_url: "F:\\Art\\outro.mov",
          },
        },
        active_media_reference_key: "DEFAULT_MEDIA",
      },
    ],
    kind: "Video",
  };

  const otio = {
    OTIO_SCHEMA: "Timeline.1",
    metadata: {
      Resolve_OTIO: {
        "Resolve OTIO Meta Version": "1.0",
      },
    },
    name: "",
    global_start_time: {
      OTIO_SCHEMA: "RationalTime.1",
      rate: new Float(60),
      value: new Float(0),
    },
    tracks: {
      OTIO_SCHEMA: "Stack.1",
      metadata: {},
      name: "",
      source_range: null,
      effects: [],
      markers: [],
      enabled: true,
      children: [videoTrack, overlayTrack, ...audioTracks],
    },
  };

  return floatJsonSerializer(otio, null, 2);
}

function convertStreamToCutSequence(stream: Stream): ConvertedCut[] {
  function fn(
    acc: { elapsed: number; clips: ConvertedCut[] },
    clip: VideoClip
  ): { elapsed: number; clips: ConvertedCut[] } {
    return {
      clips: [
        ...acc.clips,
        {
          start: acc.elapsed,
          end: acc.elapsed + clip.duration,
        },
      ],
      elapsed: acc.elapsed + clip.duration,
    };
  }

  return stream.video_clips.reduce(fn, {
    clips: [],
    elapsed: 0,
  }).clips;
}

export function generateChildren(
  episode: ConvertedEpisode,
  stream: Stream
): InternalTrack[] {
  if (episode.tracks.length === 0) {
    console.warn("No tracks to export");
    return [];
  }

  if (stream.video_clips.length === 0) {
    console.warn("No video clips to export");
    return [];
  }

  const cutSequence = convertStreamToCutSequence(stream);

  return episode.tracks.flatMap<InternalTrack>((cut: ConvertedCut) => {
    const mediaStart = findMediaClipCursorStart(cutSequence, cut.start);
    const mediaEnd = findMediaClipCursorEnd(cutSequence, cut.end);

    if (!mediaStart || !mediaEnd) {
      console.warn("Could not find media clips for cut", cut);
      return [];
    }

    if (sameMediaClip(mediaStart, mediaEnd)) {
      return [
        {
          sourcePath: stream.video_clips[mediaStart.clipIndex].uri,
          sourceStartFrames: mediaStart.time * FPS,
          durationFrames: (cut.end - cut.start) * FPS,
          totalMediaDurationFrames: (cut.end - cut.start) * FPS,
        },
      ];
    }

    const mediaIntermediates = findMediaClipCursors(
      cutSequence,
      mediaStart,
      mediaEnd
    );

    return [mediaStart, ...mediaIntermediates, mediaEnd].map((cursor) =>
      convertMediaClipCursorToInternalTrack(stream.video_clips, cursor)
    );
  });
}

/**
 * Exports the provided episode and stream data to an OTIO file.
 *
 * @param episode The episode data to export.
 * @param stream  The stream data to souce the media files from.
 * @returns       The OTIO file as a string.
 */
export default function exportOTIO(episode: Episode, stream: Stream): string {
  const convertedEp: ConvertedEpisode = {
    title: episode.title,
    description: episode.description,
    tracks: episode.tracks.map((cut) => ({
      start: parseIntoSeconds(cut.start),
      end: parseIntoSeconds(cut.end),
    })),
  };
  return internalToOTIO(generateChildren(convertedEp, stream));
}
