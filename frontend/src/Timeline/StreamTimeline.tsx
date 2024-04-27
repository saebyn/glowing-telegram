import { FC } from "react";
import SegmentSelector, { Segment } from "./SegmentSelector";
import DensityLine from "./DensityLine";

export interface DataStreamDataElement {
  start: number;
  end: number;
  density?: number;
}

interface StreamTimelineProps {
  start: number;
  end: number;

  segments: Array<Segment>;
  onUpdateSegment: (_segment: Segment) => void;

  dataStreams: Array<{
    name: string;
    data: Array<DataStreamDataElement>;
    color: [number, number, number];
  }>;
}

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
const StreamTimeline: FC<StreamTimelineProps> = ({
  start,
  end,
  segments,
  dataStreams,
  onUpdateSegment,
}) => {
  const unitlessWidth = end - start;

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
          boundsStart={start}
          boundsEnd={end}
          onUpdateSegment={onUpdateSegment}
          handleWidth={unitlessWidth * 0.02}
        />
      </div>

      {dataStreams.map(({ name, data, color }) => (
        <div style={{ flex: 1 }} key={name} title={name}>
          <DensityLine data={data} start={start} end={end} color={color} />
        </div>
      ))}
    </div>
  );
};

export default StreamTimeline;
