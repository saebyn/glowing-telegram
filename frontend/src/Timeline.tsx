/**
 * This is a component that displays a timeline of segments (aka periods of time). The timeline is a horizontal line with segments displayed as colored blocks. The segments are displayed in chronological order, and the timeline is scrollable if there are too many segments to fit in the viewport.
 *
 * The timeline is implemented as a React component. The timeline component takes a list of segments as a prop, and renders the segments as colored blocks on the timeline. The timeline component also handles scrolling and zooming of the timeline. The elements of the timeline are SVG elements.
 */
import React, { useRef, useEffect, useState } from "react";
import { styled } from "@mui/material/styles";

interface Segment {
  start: number;
  end: number;
}

interface TimelineProps {
  segments: Segment[];
  duration: number;
  className?: string;
  onChange?: (selectedSegmentIndices: number[]) => void;
}

const Timeline = ({
  segments,
  duration,
  className,
  onChange,
}: TimelineProps) => {
  const timelineRef = useRef<SVGSVGElement>(null);
  const [panTime, setPanTime] = useState(duration / 2);
  const [isPanning, setIsPanning] = useState(false);
  const [zoom, setZoom] = useState(1);

  const [selectedSegmentIndices, setSelectedSegmentIndices] = useState<
    number[]
  >([]);

  const aspectRatio = 20;
  const viewBoxHeight = duration / aspectRatio;
  const viewBoxWidth = duration;

  /* useEffect(() => {
    const handleResize = () => {
      if (timelineRef.current) {
        setWidth(timelineRef.current.clientWidth);
        setHeight(timelineRef.current.clientHeight);
      }
    };

    handleResize();

    window.addEventListener("resize", handleResize);
    return () => {
      window.removeEventListener("resize", handleResize);
    };
  }, []); */

  const handleWheel = (event: WheelEvent) => {
    event.preventDefault();

    const zoomFactor = 1 - event.deltaY / 1000;

    setZoom(Math.max(zoom * zoomFactor, 1.0));
  };

  useEffect(() => {
    if (timelineRef.current) {
      timelineRef.current.addEventListener("wheel", handleWheel, {
        passive: false,
      });
    }

    return () => {
      if (timelineRef.current) {
        timelineRef.current.removeEventListener("wheel", handleWheel);
      }
    };
  }, [handleWheel]);

  const handleSegmentClick = (index: number) => {
    if (selectedSegmentIndices.includes(index)) {
      setSelectedSegmentIndices(
        selectedSegmentIndices.filter((i) => i !== index)
      );
    } else {
      setSelectedSegmentIndices([...selectedSegmentIndices, index]);
    }

    if (onChange) {
      onChange(selectedSegmentIndices);
    }
  };

  const handlePointerDown = () => {
    setIsPanning(true);
  };

  const handlePointerUp = () => {
    setIsPanning(false);
  };

  const handlePointerMove = (event: React.PointerEvent<SVGSVGElement>) => {
    if (isPanning) {
      const dpx = event.movementX * 10 * zoom;
      const dt = (duration / viewBoxWidth) * dpx;
      setPanTime(Math.max(-duration / 4, Math.min(duration / 4, panTime + dt)));
    }
  };

  const handlePointerLeave = () => {
    setIsPanning(false);
  };

  return (
    <div className={className}>
      <svg
        ref={timelineRef}
        className={LabeledClasses.root}
        viewBox={`0 0 ${viewBoxWidth} ${viewBoxHeight}`}
        onPointerDown={handlePointerDown}
        onPointerUp={handlePointerUp}
        onPointerMove={handlePointerMove}
        onPointerLeave={handlePointerLeave}
      >
        <g
          className={LabeledClasses.timeline}
          transform={`translate(${
            panTime - duration / 2
          }, 0) scale(${zoom}, 1)`}
        >
          {segments.map((segment, index) => {
            const x1 = segment.start;
            const x2 = segment.end;
            const y1 = 0;
            const y2 = viewBoxHeight;
            return (
              <rect
                className={LabeledClasses.segment}
                onClick={() => handleSegmentClick(index)}
                key={index}
                x={x1}
                y={y1}
                width={x2 - x1}
                height={y2 - y1}
                fill={
                  selectedSegmentIndices.includes(index) ? "yellow" : "blue"
                }
              />
            );
          })}
        </g>
      </svg>
    </div>
  );
};

const PREFIX = "Timeline";

const LabeledClasses = {
  root: `${PREFIX}-root`,
  segment: `${PREFIX}-segment`,
  timeline: `${PREFIX}-timeline`,
};

export default styled(Timeline)({
  cursor: "grab",

  [`& .${LabeledClasses.root}`]: {
    overflowX: "scroll",
    border: "1px solid black",
  },

  [`& .${LabeledClasses.segment}`]: {
    cursor: "pointer",
  },

  [`& .${LabeledClasses.timeline}`]: {},
});
