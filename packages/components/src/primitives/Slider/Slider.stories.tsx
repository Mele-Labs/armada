import { useState } from "react";
import type { Meta, StoryObj } from "@storybook/react-vite";
import { expect, userEvent, within } from "storybook/test";
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

/** The arrow keys step it, and the step's name is its value. */
export const Steps: Story = {
  render: () => <Held />,
  play: async ({ canvasElement }) => {
    const slider = within(canvasElement).getByRole("slider", { name: "Effort" });
    await userEvent.type(slider, "{arrowright}");
    await expect(slider).toHaveAttribute("aria-valuetext", "xhigh");
  },
};
