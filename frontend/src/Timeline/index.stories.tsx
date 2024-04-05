import type { Meta, StoryObj } from "@storybook/react";
import Timeline from ".";
import { fn } from "@storybook/test";

const meta = {
  title: "Timeline",
  component: Timeline,
  tags: ["autodocs"],
  argTypes: {
    onToggleSegment: { control: "function" },
    onUpdateSegment: { control: "function" },
    selectedSegmentIndices: { control: "array" },
    segments: { control: "object" },
    duration: { control: "number" },
  },
  args: {
    segments: [],
    duration: 0,
    onToggleSegment: fn(),
    onUpdateSegment: fn(),
    selectedSegmentIndices: new Map(),
  },
} satisfies Meta<typeof Timeline>;

export default meta;

type Story = StoryObj<typeof meta>;

export const Empty: Story = {
  args: {
    segments: [],
    duration: 0,
    onToggleSegment: fn(),
    selectedSegmentIndices: new Map(),
  },
};

export const WithSegments: Story = {
  args: {
    segments: [
      { start: 0, end: 10 },
      { start: 20, end: 30 },
    ],
    duration: 30,
    onToggleSegment: fn(),
    selectedSegmentIndices: new Map(),
  },
};

/**
 * This story demonstrates the Timeline component with a usual segments
 * from one of my 3 hour twitch streams, where there is about 5 minutes of
 * silence at the beginning, then a 3 minute break every hour.
 *
 * The silence and both breaks are represented as one of the three segments
 * in the timeline.
 */
export const WithUsualStreamSegments: Story = {
  args: {
    segments: [
      { start: 0, end: 300 },
      { start: 3600, end: 3900 },
      { start: 7200, end: 7500 },
    ],
    duration: 10800,
    onToggleSegment: fn(),
    selectedSegmentIndices: new Map([
      [0, false],
      [1, true],
      [2, false],
    ]),
  },
};
