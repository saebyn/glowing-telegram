import { FC } from "react";
import SegmentSelector from "./SegmentSelector";
import DensityLine from "./DensityLine";

interface StreamTimelineProps {}

/**
 * StreamTimeline is a component that displays a timeline of a stream with
 * segments and density lines.
 *
 * Components heirarchy:
 * - `StreamTimeline`
 *  - `SegmentSelector`
 *  - `DensityLine`
 *  - `DensityLine`
 *  - `Brush`
 *  - `Actions`
 *  - `Legend`
 *
 */
const StreamTimeline: FC<StreamTimelineProps> = () => {
  const boundsStart = 0;
  const boundsEnd = 30;
  const segments = [
    { id: 1, start: 0, end: 10 },
    { id: 2, start: 10, end: 20 },
  ];

  const silenceDetectionSegments = [
    { start: 0, end: 1, density: 0.5 },
    { start: 1, end: 2, density: 0.8 },
    { start: 2, end: 3, density: 0.5 },
    { start: 3, end: 4, density: 0.8 },
    { start: 4, end: 5, density: 0.5 },
    { start: 5, end: 6, density: 0.8 },
    { start: 6, end: 7, density: 0.5 },
    { start: 7, end: 8, density: 0.8 },
    { start: 8, end: 9, density: 0.5 },
    { start: 9, end: 10, density: 0.8 },
    { start: 10, end: 11, density: 0.5 },
    { start: 11, end: 12, density: 0.8 },
    { start: 12, end: 13, density: 0.5 },
    { start: 13, end: 14, density: 0.8 },
    { start: 14, end: 15, density: 0.5 },
    { start: 15, end: 16, density: 0.8 },
    { start: 16, end: 17, density: 0.5 },
    { start: 17, end: 18, density: 0.8 },
    { start: 18, end: 19, density: 0.5 },
    { start: 19, end: 20, density: 0.8 },
    { start: 20, end: 21, density: 0.5 },
    { start: 21, end: 22, density: 0.8 },
    { start: 22, end: 23, density: 0.5 },
    { start: 23, end: 24, density: 0.8 },
    { start: 24, end: 25, density: 0.5 },
    { start: 25, end: 26, density: 0.8 },
    { start: 26, end: 27, density: 0.5 },
    { start: 27, end: 28, density: 0.8 },
    { start: 28, end: 29, density: 0.5 },
    { start: 29, end: 30, density: 0.8 },
  ];

  const transcriptionSegments = [
    { start: 0, end: 2, density: 0.1 },
    { start: 2, end: 4, density: 0.2 },
    { start: 4, end: 6, density: 0.1 },
    { start: 6, end: 8, density: 0.2 },
    { start: 8, end: 10, density: 0.1 },
    { start: 10, end: 12, density: 0.2 },
    { start: 12, end: 14, density: 0.1 },
    { start: 14, end: 16, density: 0.2 },
    { start: 16, end: 18, density: 0.1 },
    { start: 18, end: 20, density: 0.2 },
    { start: 20, end: 22, density: 0.1 },
    { start: 22, end: 24, density: 0.2 },
    { start: 24, end: 26, density: 0.1 },
    { start: 26, end: 28, density: 0.2 },
    { start: 28, end: 30, density: 0.1 },
  ];

  return (
    <div
      style={{
        width: "calc(100% - 32px)", // "100% - 2 * 16px
        height: "150px",
        position: "relative",

        display: "flex",
        flexDirection: "column",
        alignItems: "stretch",
        gap: "16px",
        margin: "16px",
        paddingTop: "16px",
        paddingBottom: "16px",
      }}
    >
      <div
        style={{
          position: "absolute",
          top: 0,
          left: 0,
          bottom: 0,
          right: 0,
        }}
      >
        <SegmentSelector
          segments={segments}
          boundsStart={boundsStart}
          boundsEnd={boundsEnd}
          onUpdateSegment={(id, segment) => {
            console.log(`Segment ${id} updated:`, segment);
          }}
        />
      </div>

      <div style={{ flex: 1 }}>
        <DensityLine
          data={silenceDetectionSegments}
          start={boundsStart}
          end={boundsEnd}
          color={[0, 0, 255]}
        />
      </div>
      <div style={{ flex: 1 }}>
        <DensityLine
          data={transcriptionSegments}
          start={boundsStart}
          end={boundsEnd}
          color={[255, 0, 0]}
        />
      </div>
    </div>
  );
};

export default StreamTimeline;
