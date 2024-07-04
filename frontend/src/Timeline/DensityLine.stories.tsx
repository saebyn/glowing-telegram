import type { Meta, StoryObj } from "@storybook/react";
import DensityLine from "./DensityLine";

const meta = {
  title: "DensityLine",
  component: DensityLine,
  tags: ["autodocs"],
  argTypes: {
    start: { control: "number" },
    end: { control: "number" },
    data: { control: "array" },
    color: { control: "array" },
    transitionMargin: {
      control: {
        type: "range",
        min: 0,
        max: 10,
        step: 0.1,
      },
      defaultValue: 2,
      description:
        "The margin around each period where the density transitions to the next period.",
    },
  },
  args: {},
} satisfies Meta<typeof DensityLine>;

export default meta;

type Story = StoryObj<typeof meta>;

export const Empty: Story = {
  args: {
    start: 0,
    end: 10,
    data: [],
  },
};

export const SinglePeriod: Story = {
  args: {
    start: 0,
    end: 10,
    data: [{ start: 0, end: 10, density: 0.5 }],
  },
};

export const MultiplePeriods: Story = {
  args: {
    start: 0,
    end: 200,
    data: [
      { start: 0, end: 10, density: 0.5 },
      { start: 10, end: 20, density: 0.8 },
      { start: 20, end: 30, density: 0.3 },
      { start: 30, end: 40, density: 0.6 },
      { start: 40, end: 50, density: 0.9 },
      { start: 50, end: 60, density: 0.2 },
      { start: 60, end: 70, density: 0.7 },
      { start: 70, end: 80, density: 0.4 },
      { start: 80, end: 90, density: 0.1 },
      { start: 90, end: 100, density: 0.8 },
    ],
  },
};

export const CustomColor: Story = {
  args: {
    start: 0,
    end: 100,
    data: [
      { start: 0, end: 10, density: 0 },
      { start: 10, end: 20, density: 0.9 },
      { start: 20, end: 30, density: 0.3 },
      { start: 30, end: 40, density: 0.6 },
      { start: 40, end: 50, density: 0.9 },
      { start: 50, end: 60, density: 0.2 },
      { start: 60, end: 70, density: 0.7 },
      { start: 70, end: 80, density: 0.4 },
      { start: 80, end: 90, density: 0.1 },
      { start: 90, end: 100, density: 0.8 },
    ],
    color: [255, 0, 0],
  },
};

export const CustomStartAndEnd: Story = {
  args: {
    start: 10,
    end: 90,
    data: [
      { start: 10, end: 20, density: 0.5 },
      { start: 20, end: 30, density: 0.8 },
      { start: 30, end: 40, density: 0.3 },
      { start: 40, end: 50, density: 0.6 },
      { start: 50, end: 60, density: 0.9 },
      { start: 60, end: 70, density: 0.2 },
      { start: 70, end: 80, density: 0.7 },
      { start: 80, end: 90, density: 0.4 },
    ],
  },
};

export const VariableDensityAndPeriods: Story = {
  args: {
    start: 0,
    end: 100,
    data: [
      { start: 0, end: 10, density: 0.5 },
      { start: 10, end: 11, density: 0.8 },
      { start: 11, end: 22, density: 0.3 },
      { start: 22, end: 33, density: 0.6 },
      { start: 33, end: 34, density: 0.9 },
      { start: 34, end: 45, density: 0.2 },
      { start: 45, end: 56, density: 0.7 },
      { start: 56, end: 67, density: 0.4 },
      { start: 67, end: 78, density: 0.1 },
      { start: 78, end: 100, density: 0 },
    ],
  },
};

export const DataOutsideOfStartAndEnd: Story = {
  args: {
    start: 10,
    end: 90,
    data: [
      { start: 0, end: 10, density: 0.9 },
      { start: 10, end: 90, density: 0.0 },
      { start: 90, end: 100, density: 0.9 },
    ],
  },
};
