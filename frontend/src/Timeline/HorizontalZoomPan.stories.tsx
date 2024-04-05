import type { Meta, StoryObj } from "@storybook/react";
import HorizontalZoomPan from "./HorizontalZoomPan";

const meta = {
  title: "HorizontalZoomPan",
  component: HorizontalZoomPan,
  tags: ["autodocs"],
  argTypes: {
    children: { control: "function" },
  },
  args: {},
} satisfies Meta<typeof HorizontalZoomPan>;

export default meta;

type Story = StoryObj<typeof meta>;

export const Empty: Story = {
  args: {
    children: (props: {
      viewStart: number;
      viewEnd: number;
      resetView: () => void;
    }) => (
      <div
        style={{
          width: "100%",
          height: "100%",
          border: "1px solid black",
          padding: "50px",
        }}
      >
        <div>View start: {props.viewStart}</div>
        <div>View end: {props.viewEnd}</div>
        <button onClick={props.resetView}>Reset View</button>
      </div>
    ),
  },
};
