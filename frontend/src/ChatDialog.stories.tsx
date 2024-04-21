import type { Meta, StoryObj } from "@storybook/react";
import ChatDialog from "./ChatDialog";
import { fn } from "@storybook/test";

const meta = {
  title: "ChatDialog",
  component: ChatDialog,
  tags: ["autodocs"],
  argTypes: {
    open: { control: "boolean" },
    onChat: { control: "function" },
    onChange: { control: "function" },
    job: { control: "string" },
    transcript: { control: "string" },
    context: { control: "string" },
  },
  args: {
    open: true,
    job: "",
    transcript: "",
    context: "",
    onChat: fn((msgs) => Promise.resolve(msgs)),
    onChange: fn(),
  },
} satisfies Meta<typeof ChatDialog>;

export default meta;

type Story = StoryObj<typeof meta>;

export const Empty: Story = {};

export const WithMessages: Story = {
  args: {
    job: "Job title",
    transcript: "Transcript",
    context: "Context",
    onChat: fn(async (msgs) => {
      return [
        ...msgs,
        {
          content: "Assistant message",
          role: "assistant" as const,
        },
      ];
    }),
  },
};
