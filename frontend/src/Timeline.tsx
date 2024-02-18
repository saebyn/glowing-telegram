/**
 * This is a component that displays a timeline of segments (aka periods of time). The timeline is a horizontal line with segments displayed as colored blocks. The segments are displayed in chronological order, and the timeline is scrollable if there are too many segments to fit in the viewport.
 *
 * The timeline is implemented as a React component. The timeline component takes a list of segments as a prop, and renders the segments as colored blocks on the timeline. The timeline component also handles scrolling and zooming of the timeline. The elements of the timeline are SVG elements.
 */
import React, { useRef, useEffect, useState } from "react";

interface Segment {
  start: number;
  end: number;
}

interface TimelineProps {
  segments: Segment[];
}

const Timeline = ({ segments }: TimelineProps) => {
  const timelineRef = useRef<SVGSVGElement>(null);
  const [width, setWidth] = useState(0);
  const [height, setHeight] = useState(0);
  const [scrollX, setScrollX] = useState(0);
  const [zoom, setZoom] = useState(1);

  useEffect(() => {
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
  }, []);

  const handleWheel = (event: React.WheelEvent<SVGSVGElement>) => {
    event.preventDefault();
    setZoom(zoom * (1 - event.deltaY / 1000));
  };

  const handleScroll = (event: React.UIEvent<SVGSVGElement>) => {
    setScrollX(event.currentTarget.scrollLeft);
  };

  return (
    <svg
      ref={timelineRef}
      onWheel={handleWheel}
      onScroll={handleScroll}
      width={width}
      height={height}
      style={{ overflowX: "scroll" }}
    >
      <g transform={`translate(${scrollX}, 0) scale(${zoom}, 1)`}>
        {segments.map((segment, index) => {
          const x1 = segment.start;
          const x2 = segment.end;
          const y1 = 0;
          const y2 = height;
          return (
            <rect
              key={index}
              x={x1}
              y={y1}
              width={x2 - x1}
              height={y2 - y1}
              fill="blue"
            />
          );
        })}
      </g>
    </svg>
  );
};

export default Timeline;
