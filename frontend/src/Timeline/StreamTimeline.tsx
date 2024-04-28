import { FC } from "react";
import SegmentSelector, { Segment } from "./SegmentSelector";
import DensityLine from "./DensityLine";
import { formatDuration } from "../isoDuration";
import Button from "@mui/material/Button";

export interface DataStreamDataElement {
  start: number;
  end: number;
  density?: number;
}

interface StreamTimelineProps {
  start: number;
  end: number;

  segments: Array<Segment>;
  onUpdate: (_segments: Segment[]) => void;
  onReset: () => void;

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
  onUpdate,
  onReset,
}) => {
  const unitlessWidth = end - start;

  const handleAddSegment = () => {
    const newSegment = {
      id: segments.length,
      start: start,
      end: end,
    };
    onUpdate([...segments, newSegment]);
  };

  const handleRemoveSegment = (segment: Segment) => {
    onUpdate(segments.filter((s) => s.id !== segment.id));
  };

  const handleUpdateSegment = (segment: Segment) => {
    onUpdate(segments.map((s) => (s.id === segment.id ? segment : s)));
  };

  return (
    <div>
      <div style={{ display: "flex", flexDirection: "row" }}>
        <div style={{ flex: 1 }}>
          <Button onClick={handleAddSegment}>Add</Button>
          <Button onClick={onReset}>Reset</Button>
        </div>

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
              inset: "0",
              height: "100px",
            }}
          >
            <SegmentSelector
              segments={segments}
              boundsStart={start}
              boundsEnd={end}
              onUpdateSegment={handleUpdateSegment}
              handleWidth={unitlessWidth * 0.02}
            />
          </div>

          {dataStreams.map(({ name, data, color }) => (
            <div style={{ flex: 1 }} key={name} title={name}>
              <DensityLine
                data={data}
                start={start}
                end={end}
                color={color}
                transitionMargin={0}
              />
            </div>
          ))}

          <TimelineLegend start={start} end={end} />
        </div>
      </div>
      <div style={{ flex: 1 }}>
        <table style={{ width: "100%", marginTop: "16px" }}>
          <thead>
            <tr>
              <th>Start</th>
              <th>End</th>
              <th>Duration</th>
              <th>Actions</th>
            </tr>
          </thead>
          <tbody>
            {segments.map((segment) => (
              <tr key={segment.start}>
                <td>{formatDuration(segment.start)}</td>
                <td>{formatDuration(segment.end)}</td>
                <td>{formatDuration(segment.end - segment.start)}</td>
                <td>
                  <Button onClick={() => handleRemoveSegment(segment)}>
                    Remove
                  </Button>
                </td>
              </tr>
            ))}
          </tbody>
        </table>
      </div>
    </div>
  );
};

interface TimelineLegendProps {
  start: number;
  end: number;
}

const TimelineLegend: FC<TimelineLegendProps> = ({ start, end }) => {
  const timeIntervals = [15, 60, 300, 900, 3600, 14400, 86400];

  // Find the largest time interval that fits at least 3 times in the timeline
  const interval =
    timeIntervals.findLast((interval) => (end - start) / interval > 3) || 1;

  const intervals = Array.from(
    { length: Math.floor((end - start) / interval) - 1 },
    (_, i) => start + (i + 1) * interval
  );

  return (
    <div
      style={{
        pointerEvents: "none",
        display: "flex",
        flexDirection: "row",
        justifyContent: "space-between",
      }}
    >
      <div>{formatDuration(Math.floor(start))}</div>

      {intervals.map((time) => (
        <div key={time}>{formatDuration(time)}</div>
      ))}

      <div>{formatDuration(Math.floor(end))}</div>
    </div>
  );
};

export default StreamTimeline;
