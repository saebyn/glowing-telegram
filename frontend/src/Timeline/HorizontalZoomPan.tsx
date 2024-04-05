import React, { useCallback, useEffect } from "react";

export interface ZoomPanProps {
  /**
   * The relative position of the start of the view, between 0 and 1.
   */
  viewStart: number;
  /**
   * The relative position of the end of the view, between 0 and 1.
   */
  viewEnd: number;
  /**
   * Call this function to reset the view to its initial state.
   */
  resetView: () => void;
}

interface HorizontalZoomPanProps {
  className?: string;
  children: (_props: ZoomPanProps) => React.ReactNode;
  minZoom?: number;
  maxZoom?: number;
  panAcceleration?: number;
}

// Our internal pan time is a value between 0 and 1, where 0 is the minimum
// value and 1 is the maximum value.
const initialPan = {
  pan: 0.5,
  panAcceleration: 0,
};

const HorizontalZoomPan = (props: HorizontalZoomPanProps) => {
  // The maximum acceleration of the pan when dragging, in seconds per pixel.
  const maxPanAcceleration = 0.01;

  const [{ pan }, setPan] = React.useState(initialPan);
  const [isPanning, setIsPanning] = React.useState(false);
  const [zoom, setZoom] = React.useState(1);

  const containerRef = React.useRef<HTMLDivElement>(null);

  const minZoom = props.minZoom ?? 1;
  const maxZoom = props.maxZoom ?? Infinity;
  const basePanAcceleration = props.panAcceleration ?? 0.0001;

  const handleWheel = useCallback(
    (event: WheelEvent) => {
      event.preventDefault();

      // Zoom in or out based on the scroll direction
      const zoomFactor = 1 - event.deltaY / 1000;

      // Find the relative position of the cursor in the container
      const container = containerRef.current;
      if (!container) {
        return;
      }

      const containerWidth = container.clientWidth;
      const cursorX =
        event.clientX -
        (container.getBoundingClientRect().left + containerWidth / 2);

      // Calculate the new pan based on the cursor position
      setPan(({ pan: existingPan, panAcceleration }) =>
        calculateNewPan(
          cursorX / containerWidth,
          existingPan,
          zoom,
          panAcceleration,
          maxPanAcceleration,
          basePanAcceleration
        )
      );

      setZoom((prevZoom) => {
        const newZoom = Math.max(
          minZoom,
          Math.min(maxZoom, prevZoom * zoomFactor)
        );

        return newZoom;
      });
    },
    [minZoom, maxZoom, zoom, basePanAcceleration, maxPanAcceleration]
  );

  const handleMouseDown = () => {
    setIsPanning(true);
  };

  useEffect(() => {
    const timeline = containerRef.current;

    if (timeline) {
      timeline.addEventListener("wheel", handleWheel, {
        passive: false,
      });
    }

    const handleMouseUpOutside = () => {
      setIsPanning(false);
    };

    const handlePan = (event: MouseEvent) => {
      event.preventDefault();

      if (!isPanning) {
        return;
      }

      const container = containerRef.current;
      if (!container) {
        return;
      }

      const containerWidth = container.clientWidth;

      setPan(({ pan: existingPan, panAcceleration }) =>
        calculateNewPan(
          -event.movementX / containerWidth,
          existingPan,
          zoom,
          panAcceleration,
          maxPanAcceleration,
          basePanAcceleration
        )
      );
    };

    document.addEventListener("mouseup", handleMouseUpOutside);
    document.addEventListener("mousemove", handlePan);

    return () => {
      document.removeEventListener("mouseup", handleMouseUpOutside);
      document.removeEventListener("mousemove", handlePan);
      if (timeline) {
        timeline.removeEventListener("wheel", handleWheel);
      }
    };
  }, [
    handleWheel,
    isPanning,
    maxPanAcceleration,
    pan,
    zoom,
    basePanAcceleration,
  ]);

  // Reset pan and zoom when the component is unmounted
  React.useEffect(() => {
    return () => {
      setPan(initialPan);
      setZoom(1);
    };
  }, []);

  function resetView() {
    setPan(initialPan);
    setZoom(1);
  }

  const viewableRange = 1 / zoom;
  const viewStart = Math.max(0, pan - viewableRange / 2);
  const viewEnd = Math.min(1, pan + viewableRange / 2);

  return (
    <div
      ref={containerRef}
      className={props.className}
      onMouseDown={handleMouseDown}
    >
      {props.children({
        viewStart,
        viewEnd,
        resetView,
      })}
    </div>
  );
};

function calculateNewPan(
  pixelDelta: number,
  pan: number,
  zoom: number,
  panAcceleration: number,
  maxPanAcceleration: number,
  basePanAcceleration: number
): {
  pan: number;
  panAcceleration: number;
} {
  const newPanAcceleration = Math.max(
    Math.min(maxPanAcceleration, Math.abs(pixelDelta) * panAcceleration),
    basePanAcceleration
  );

  const newPan = pan + pixelDelta * zoom * newPanAcceleration;

  return {
    pan: Math.max(0, Math.min(1, newPan)),
    panAcceleration: newPanAcceleration,
  };
}

export default HorizontalZoomPan;
