import { useState } from "react";
import type { Meta, StoryObj } from "@storybook/react-vite";
import { expect, fireEvent, within } from "storybook/test";
import { Slider } from "./Slider";

const meta: Meta<typeof Slider> = {
  title: "Primitives/Slider",
  component: Slider,
};
export default meta;

type Story = StoryObj<typeof Slider>;

const EFFORTS = ["default", "low", "medium", "high", "xhigh", "max"];

function Held() {
  const [value, setValue] = useState("high");
  return (
    <div style={{ width: "var(--w-guide-list)" }}>
      <Slider aria-label="Effort" steps={EFFORTS} value={value} onChange={setValue} />
    </div>
  );
}

/** A step's name is its value. A synthetic key moves no native range, so the step is set as the input does. */
export const Steps: Story = {
  render: () => <Held />,
  play: async ({ canvasElement }) => {
    const slider = within(canvasElement).getByRole("slider", { name: "Effort" });
    await expect(slider).toHaveAttribute("aria-valuetext", "high");
    fireEvent.change(slider, { target: { value: "4" } });
    await expect(slider).toHaveAttribute("aria-valuetext", "xhigh");
  },
};
