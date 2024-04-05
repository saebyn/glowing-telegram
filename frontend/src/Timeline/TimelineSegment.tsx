import React, { useState } from "react";
import { Segment } from "./types";
import { styled } from "@mui/material/styles";

interface TimelineSegmentProps {
  segment: Segment;
  height: number;
  index: number;
  selected: boolean;
  viewStart: number;
  duration: number;
  viewBoxWidth: number;
  viewableDuration: number;
  onToggleSegment: (_segmentIndex: number) => void;
  onUpdateSegment: (_segmentIndex: number, _segment: Segment) => void;
}

const TimelineSegment = ({
  className,
  segment,
  selected,
  height,
  index,
  viewBoxWidth,
  viewStart,
  duration,
  viewableDuration,
  onToggleSegment,
  onUpdateSegment,
}: TimelineSegmentProps & { className?: string }) => {
  function x(time: number) {
    return ((time - viewStart * duration) / viewableDuration) * viewBoxWidth;
  }

  const x1 = segment.start;
  const x2 = segment.end;
  const width = x(x2) - x(x1);

  const [dragging, setDragging] = useState<{
    index: number;
    handle: "start" | "end";
  } | null>(null);

  const handleMouseDown = (event: React.MouseEvent<SVGRectElement>) => {
    const index = parseInt(
      event.currentTarget.getAttribute("data-segment-index") as string,
      10
    );
    const handle = event.currentTarget.getAttribute("data-handle") as
      | "start"
      | "end";

    setDragging({ index, handle });
  };

  const handleMouseUp = () => {
    setDragging(null);
  };

  const handleMouseMove = (event: React.MouseEvent<SVGRectElement>) => {
    if (dragging) {
      const x1 = segment.start;
      const x2 = segment.end;
      const offset = (event.clientX / viewBoxWidth) * viewableDuration;

      if (dragging.handle === "start") {
        const newStart = x1 - offset;
        const newEnd = x2;

        onUpdateSegment(dragging.index, { start: newStart, end: newEnd });
      } else {
        const newStart = x1;
        const newEnd = x2 - offset;

        onUpdateSegment(dragging.index, { start: newStart, end: newEnd });
      }
    }
  };

  if (selected) {
    const dragHandleWidth = width / 20;

    return (
      <g className={className}>
        <rect
          data-segment-index={index}
          data-handle="start"
          onMouseDown={handleMouseDown}
          onMouseUp={handleMouseUp}
          onMouseMove={handleMouseMove}
          className={LabeledClasses.dragHandle}
          x={x(x1) - dragHandleWidth}
          y={0}
          width={dragHandleWidth}
          height={height}
          fill={"goldenrod"}
        />
        <rect
          onClick={() => onToggleSegment(index)}
          x={x(x1)}
          y={0}
          width={width}
          height={height}
          fill={"yellow"}
        />
        <rect
          data-segment-index={index}
          data-handle="end"
          onMouseDown={handleMouseDown}
          onMouseUp={handleMouseUp}
          onMouseMove={handleMouseMove}
          className={LabeledClasses.dragHandle}
          x={x(x2)}
          y={0}
          width={dragHandleWidth}
          height={height}
          fill={"goldenrod"}
        />
      </g>
    );
  } else {
    return (
      <g className={className}>
        <rect
          onClick={() => onToggleSegment(index)}
          x={x(x1)}
          y={0}
          width={width}
          height={height}
          fill={"blue"}
        />
      </g>
    );
  }
};

const PREFIX = "TimelineSegment";

const LabeledClasses = {
  dragHandle: `${PREFIX}-dragHandle`,
};

export default styled(TimelineSegment)({
  cursor: "pointer",

  [`& .${LabeledClasses.dragHandle}`]: {
    cursor: "ew-resize",
  },
});
