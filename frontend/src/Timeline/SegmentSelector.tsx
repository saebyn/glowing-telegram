import React, { FC, useState, useEffect } from "react";

/**
 * Segment interface
 *
 * Represents a segment of a timeline with a start and end time.
 *
 * @param id - The unique identifier of the segment.
 * @param start - The start time of the segment.
 * @param end - The end time of the segment.
 */
export interface Segment {
  id: number;
  start: number;
  end: number;
}

/**
 * SegmentSelectorProps interface
 *
 * The `SegmentSelectorProps` interface defines the props for the `SegmentSelector` component.
 *
 * @param segments - An array of `Segment` objects representing the timeline segments.
 * @param boundsStart - The start time of the timeline.
 * @param boundsEnd - The end time of the timeline.
 * @param onUpdateSegment - A function that is called when a segment is updated.
 */
interface SegmentSelectorProps {
  segments: Segment[];
  boundsStart: number;
  boundsEnd: number;
  onUpdateSegment: (_segment: Segment) => void;

  // Optional props
  handleWidth?: number;
  handleHeight?: number;
  segmentFill?: string;
  handleFill?: string;
}

/**
 * SegmentSelector component
 *
 * A controlled component that displays a list of timeline segments and allows the user to drag the start and end points of each segment to adjust its duration.
 *
 * The component takes a list of segments, a start time, an end time, and a callback function that is called when a segment is updated.
 *
 * The component renders a list of segments as shaded blocks on a timeline. The user can drag the start and end point handles rendered for each segment, to adjust its duration. When a segment is updated, the `onUpdateSegment` callback is called with the segment's id and the updated segment.
 *
 * @example
 * ```tsx
 * <SegmentSelector
 *   segments={[
 *    { id: 1, start: 0, end: 10 },
 *    { id: 2, start: 20, end: 30 },
 *   ]}
 *   boundsStart={0}
 *   boundsEnd={30}
 *   onUpdateSegment={(id, segment) => {
 *     console.log(`Segment ${id} updated:`, segment);
 *    }}
 * />
 * ```
 *
 */
const SegmentSelector: FC<SegmentSelectorProps> = ({
  segments,
  boundsStart,
  boundsEnd,
  onUpdateSegment,

  handleWidth = 0.2,
  handleHeight = 1,
  segmentFill = "rgba(0, 0, 255, 0.25)",
  handleFill = "goldenrod",
}) => {
  return (
    <svg
      viewBox={`${boundsStart} 0 ${boundsEnd} 1`}
      height="100%"
      width="100%"
      preserveAspectRatio="none"
    >
      {segments
        .filter(
          (segment) => segment.start >= boundsStart && segment.end <= boundsEnd
        )
        .map((segment) => (
          <g key={segment.id}>
            <SegmentContents
              start={segment.start}
              end={segment.end}
              fill={segmentFill}
            />
            <SegmentHandles
              segment={segment}
              onUpdateSegment={onUpdateSegment}
              boundsStart={boundsStart}
              boundsEnd={boundsEnd}
              handleWidth={handleWidth}
              handleHeight={handleHeight}
              fill={handleFill}
            />
          </g>
        ))}
    </svg>
  );
};

interface SegmentContentsProps {
  start: number;
  end: number;
  fill?: string;
}

const SegmentContents: FC<SegmentContentsProps> = ({
  start,
  end,
  fill = "blue",
}) => {
  return <rect x={start} y={0} width={end - start} height={20} fill={fill} />;
};

interface SegmentHandlesProps {
  segment: Segment;
  onUpdateSegment: (_segment: Segment) => void;
  boundsStart: number;
  boundsEnd: number;
  handleWidth?: number;
  handleHeight?: number;
  fill?: string;
}

function translateMouseEventToSvgX(
  event: MouseEvent,
  svgElement: SVGSVGElement
) {
  const svgPoint = svgElement.createSVGPoint();
  svgPoint.x = event.clientX;
  svgPoint.y = event.clientY;
  const svgPointTransformed = svgPoint.matrixTransform(
    svgElement.getScreenCTM()!.inverse()
  );
  return svgPointTransformed.x;
}

const SegmentHandles: FC<SegmentHandlesProps> = ({
  segment,
  onUpdateSegment,
  boundsStart,
  boundsEnd,
  handleWidth = 0.2,
  handleHeight = 1,
  fill = "black",
}) => {
  const [draggingHandle, setDraggingHandle] = useState<"start" | "end" | null>(
    null
  );

  const handleRef = React.createRef<SVGGElement>();
  const handleStartMouseDown = () => {
    setDraggingHandle("start");
  };

  const handleMouseUp = () => {
    setDraggingHandle(null);
  };

  const handleMouseMove = (event: MouseEvent) => {
    if (draggingHandle === null) {
      return;
    }

    if (!handleRef.current) {
      return;
    }

    const handle = handleRef.current;

    if (draggingHandle === "start") {
      const svgX = translateMouseEventToSvgX(
        event,
        handle.ownerSVGElement as SVGSVGElement
      );

      const newStart = Math.max(boundsStart, Math.min(svgX, segment.end));

      onUpdateSegment({
        id: segment.id,
        start: newStart,
        end: segment.end,
      });
    } else if (draggingHandle === "end") {
      const svgX = translateMouseEventToSvgX(
        event,
        handle.ownerSVGElement as SVGSVGElement
      );

      const newEnd = Math.min(boundsEnd, Math.max(svgX, segment.start));

      onUpdateSegment({
        id: segment.id,
        start: segment.start,
        end: newEnd,
      });
    }
  };

  const handleEndMouseDown = () => {
    setDraggingHandle("end");
  };

  // to ensure that we can continue dragging the handle even if the mouse leaves the handle, we attach the mouseup and mousemove event listeners to the window
  useEffect(() => {
    window.addEventListener("mouseup", handleMouseUp);
    window.addEventListener("mousemove", handleMouseMove);

    return () => {
      window.removeEventListener("mouseup", handleMouseUp);
      window.removeEventListener("mousemove", handleMouseMove);
    };
  });

  return (
    <g ref={handleRef}>
      <SegmentHandle
        x={segment.start - handleWidth / 2}
        y={0}
        width={handleWidth}
        height={handleHeight}
        fill={fill}
        onMouseDown={handleStartMouseDown}
      />
      <SegmentHandle
        x={segment.end - handleWidth / 2}
        y={0}
        width={handleWidth}
        height={handleHeight}
        fill={fill}
        onMouseDown={handleEndMouseDown}
      />
    </g>
  );
};

const SegmentHandle: FC<{
  x: number;
  y: number;
  width: number;
  height: number;
  fill?: string;
  onMouseDown: () => void;
}> = ({ x, y, width, height, onMouseDown, fill = "black" }) => {
  // Drag handle for the segment
  // A resize handle that allows the user to adjust the start or end time of a segment
  // It has a larger inverted triangle shape at the top and bottom to make it easier to grab
  // and the body of the handle is a narrow rectangle
  return (
    <g
      onMouseDown={onMouseDown}
      cursor={"ew-resize"}
      transform={`translate(${x}, ${y})`}
    >
      <polygon
        points={`0,0 ${width},0 ${width / 2},${height / 8}`}
        fill={fill}
      />
      <polygon
        points={`0,${height} ${width},${height} ${width / 2},${
          (7 * height) / 8
        }`}
        fill={fill}
      />
      <rect
        x={width / 2 - width / 8 / 2}
        y={height / 8 - height / 8 / 2}
        width={width / 8}
        height={(3 * height) / 4 + height / 8}
        fill={fill}
      />
    </g>
  );
};

export default SegmentSelector;
