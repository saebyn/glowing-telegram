import { FC } from "react";

interface DensityPeriod {
  start: number;
  end: number;
  density?: number;
}

/**
 * DensityLineProps interface
 *
 * The DensityLineProps interface defines the props for the DensityLine component.
 *
 * @param data - An array of DensityPeriod objects, each representing a period of time and the density of events during that period.
 * @param start - The start time of the timeline as a unitless number.
 * @param end - The end time of the timeline as a unitless number.
 * @param color - The color of the shaded area as an array of RGB values. Defaults to black.
 * @param transitionMargin - The margin around each period where the density transitions to the next period. Defaults to 2.
 */
interface DensityLineProps {
  data: DensityPeriod[];
  start: number;
  end: number;
  color?: [number, number, number];
  transitionMargin?: number;
}

/**
 * DensityLine component
 *
 * The DensityLine component displays a shaded area on a timeline to represent the density of events over time. It uses a set of SVG elements to draw the shaded area on the timeline.
 *
 * @example
 * ```tsx
 * <DensityLine
 *  data={[
 *   { start: 0, end: 10, density: 0.5 },
 *   { start: 10, end: 20, density: 0.8 },
 *  ]}
 *  start={0}
 *  end={20}
 * />
 * ```
 */
const DensityLine: FC<DensityLineProps> = ({
  data,
  start,
  end,
  color,
  transitionMargin = 2,
}) => {
  // Calculate the maximum density
  const maxDensity = Math.max(...data.map((period) => period.density || 0));

  // If no color is provided, default to black
  if (!color) {
    color = [0, 0, 0];
  }

  // Create the gradient color stops
  const colorStops = [];
  let previousPeriodEnd = start;

  for (const period of data) {
    // Skip periods that are outside the timeline
    if (period.start >= end || period.end <= start) {
      continue;
    }

    // if there is a gap between the end of the previous period and the start of this period, add a pair of color stops to transition between the two periods
    if (period.start > previousPeriodEnd) {
      const startPosition = ((previousPeriodEnd - start) / (end - start)) * 100;
      const endPosition = ((period.start - start) / (end - start)) * 100;
      colorStops.push(
        `${`rgba(${color[0]}, ${color[1]}, ${color[2]}, 0)`} ${
          startPosition + transitionMargin / 2
        }%`,
        `${`rgba(${color[0]}, ${color[1]}, ${color[2]}, 0)`} ${
          endPosition - transitionMargin / 2
        }%`
      );
    }

    // Calculate the start position of the period as a percentage of the total timeline width
    const startPosition = ((period.start - start) / (end - start)) * 100;

    // Calculate the end position of the period as a percentage of the total timeline width
    const endPosition = ((period.end - start) / (end - start)) * 100;

    // Calculate the opacity of the period based on its density
    const opacity = (period.density || 0) / maxDensity;
    colorStops.push(
      `${`rgba(${color[0]}, ${color[1]}, ${color[2]}, ${opacity})`} ${
        startPosition + transitionMargin / 2
      }%`,
      `${`rgba(${color[0]}, ${color[1]}, ${color[2]}, ${opacity})`} ${
        endPosition - transitionMargin / 2
      }%`
    );
  }

  // if there is a gap between the end of the last period and the end of the timeline, add a pair of color stops to transition to the end of the timeline
  if (data.length > 0 && data[data.length - 1].end < end) {
    const startPosition =
      ((data[data.length - 1].end - start) / (end - start)) * 100;
    const endPosition = 100;
    colorStops.push(
      `${`rgba(${color[0]}, ${color[1]}, ${color[2]}, 0)`} ${
        startPosition + transitionMargin / 2
      }%`,
      `${`rgba(${color[0]}, ${color[1]}, ${color[2]}, 0)`} ${
        endPosition - transitionMargin / 2
      }%`
    );
  }

  return (
    <div
      style={{
        pointerEvents: "none",

        width: "100%",
        height: "100%",

        background: `linear-gradient(to right, ${colorStops.join(", ")})`,
      }}
    />
  );
};

export default DensityLine;
