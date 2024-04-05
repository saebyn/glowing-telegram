/**
 * This is a component that displays a timeline of segments (aka periods of time). The timeline is a horizontal line with segments displayed as colored blocks. The segments are displayed in chronological order, and the timeline is scrollable if there are too many segments to fit in the viewport.
 *
 * The timeline is implemented as a React component. The timeline component takes a list of segments as a prop, and renders the segments as colored blocks on the timeline. The timeline component also handles scrolling and zooming of the timeline. The elements of the timeline are SVG elements.
 */
import { styled } from "@mui/material/styles";
import { Button } from "@mui/material";
import { formatDuration } from "../isoDuration";
import HorizontalZoomPan, { ZoomPanProps } from "./HorizontalZoomPan";
import TimelineSegment from "./TimelineSegment";
import { Segment } from "./types";

export interface TimelineProps {
  segments: Segment[];
  duration: number;
  className?: string;
  onToggleSegment: (_segmentIndex: number) => void;
  onUpdateSegment: (_segmentIndex: number, _segment: Segment) => void;
  selectedSegmentIndices: Map<number, boolean>;
}

const Timeline = ({
  segments,
  duration,
  className,
  onToggleSegment,
  onUpdateSegment,
  selectedSegmentIndices,
}: TimelineProps) => {
  return (
    <HorizontalZoomPan className={className} panAcceleration={0.1}>
      {(props) => (
        <TimelineView
          {...props}
          segments={segments}
          duration={duration}
          onToggleSegment={onToggleSegment}
          onUpdateSegment={onUpdateSegment}
          selectedSegmentIndices={selectedSegmentIndices}
        />
      )}
    </HorizontalZoomPan>
  );
};

type TimelineViewProps = ZoomPanProps & {
  segments: Segment[];
  duration: number;
  onToggleSegment: (_segmentIndex: number) => void;
  onUpdateSegment: (_segmentIndex: number, _segment: Segment) => void;
  selectedSegmentIndices: Map<number, boolean>;
};

const TimelineView = ({
  viewEnd,
  viewStart,
  resetView,
  segments,
  duration,
  onToggleSegment,
  onUpdateSegment,
  selectedSegmentIndices,
}: TimelineViewProps) => {
  const aspectRatio = 20;
  const viewBoxHeight = duration / aspectRatio;
  const viewBoxWidth = duration;
  const viewableDuration = (viewEnd - viewStart) * duration;
  const scaleHeight = viewBoxHeight / 10;

  const segmentsInView = segments;

  return (
    <>
      <svg
        className={LabeledClasses.root}
        viewBox={`0 0 ${viewBoxWidth} ${viewBoxHeight}`}
      >
        <g className={LabeledClasses.timeline}>
          {segmentsInView.map((segment, index) => (
            <TimelineSegment
              key={index}
              index={index}
              segment={segment}
              viewStart={viewStart}
              duration={duration}
              viewableDuration={viewableDuration}
              viewBoxWidth={viewBoxWidth}
              height={viewBoxHeight - scaleHeight - 1}
              selected={!!selectedSegmentIndices.get(index)}
              onToggleSegment={onToggleSegment}
              onUpdateSegment={onUpdateSegment}
            />
          ))}
        </g>
        <g>
          <line
            x1={((0 - viewStart * duration) / viewableDuration) * viewBoxWidth}
            y1={viewBoxHeight - scaleHeight}
            x2={duration}
            y2={viewBoxHeight - scaleHeight}
            stroke="black"
            strokeWidth="2"
          />
          {/* Show ticks, showing the time in seconds */}
          {Array(15)
            .fill(0)
            .map((_, i) => {
              return (i * viewableDuration) / 15 + viewStart * duration;
            })
            .map((seconds) => (
              <text
                key={seconds}
                x={
                  ((seconds - viewStart * duration) / viewableDuration) *
                  viewBoxWidth
                }
                y={viewBoxHeight - 3}
                fontSize={viewBoxHeight / 10}
                fill="black"
              >
                {formatDuration(seconds)}
              </text>
            ))}
        </g>
      </svg>

      <Button onClick={resetView}>Reset View</Button>
    </>
  );
};

const PREFIX = "Timeline";

const LabeledClasses = {
  root: `${PREFIX}-root`,

  timeline: `${PREFIX}-timeline`,
};

export default styled(Timeline)({
  cursor: "grab",

  [`& .${LabeledClasses.root}`]: {
    overflowX: "scroll",
    border: "1px solid black",
    padding: "0 0 10px 0",
  },

  [`& .${LabeledClasses.timeline}`]: {},
});
