import type { Meta, StoryObj } from "@storybook/react";
import DurationField from "./DurationField";
import { fn } from "@storybook/test";

const meta = {
  title: "DurationField",
  component: DurationField,
  tags: ["autodocs"],
  argTypes: {
    onChange: { action: "changed" },
    onBlur: { action: "blurred" },
    value: { control: "text" },
  },
  args: {
    value: "PT0S",
    onChange: fn(),
    onBlur: fn(),
  },
} satisfies Meta<typeof DurationField>;

export default meta;

type Story = StoryObj<typeof meta>;

export const Default: Story = {
  args: {
    value: "PT0S",
  },
};
