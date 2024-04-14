import type { Meta, StoryObj } from "@storybook/react";
import MediaPicker from "./MediaPicker";
import { fn } from "@storybook/test";

const meta = {
  title: "MediaPicker",
  component: MediaPicker,
  tags: ["autodocs"],
  argTypes: {
    onChoose: {
      action: "onChoose",
    },
    entries: { control: "array" },
    value: { control: "text" },
  },
  args: {},
} satisfies Meta<typeof MediaPicker>;

export default meta;

type Story = StoryObj<typeof meta>;

export const Empty: Story = {
  args: {
    entries: [],
    onChoose: fn(),
    value: null,
  },
};

export const WithEntries: Story = {
  args: {
    entries: [
      {
        uri: "file:local:example.mp4",
        metadata: {
          filename: "example.mp4",
          size: 1_000_000_000,
        },
      },
      {
        uri: "file:local:example2.mp4",
        metadata: {
          filename: "example2.mp4",
          size: 1_000_000_000,
        },
      },
      {
        uri: "file:local:example3.mp4",
        metadata: {
          filename: "example3.mp4",
          size: 1_000_000_000,
        },
      },
    ],
    onChoose: fn(),
    value: "file:local:example2.mp4",
  },
};
