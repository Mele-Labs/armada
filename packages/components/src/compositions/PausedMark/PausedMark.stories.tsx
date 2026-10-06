import type { Meta, StoryObj } from "@storybook/react-vite";
import { expect } from "storybook/test";
import { PausedMark } from "./PausedMark";

const meta: Meta<typeof PausedMark> = {
  title: "Compositions/Paused mark",
  component: PausedMark,
};
export default meta;

type Story = StoryObj<typeof PausedMark>;

/** A person's pause. The tooltip names the branch the work is on, and when. */
export const PausedByAPerson: Story = {
  args: { said: "Paused 4m ago: work saved on branch armada/job-2-debounce, slot released" },
  play: async ({ canvas }) => {
    const mark = canvas.getByRole("img", { name: /^Paused 4m ago: work saved on branch armada\/job-2-debounce/ });
    // An icon and its tooltip, never a phrase beside it.
    await expect(mark.querySelector("svg")).not.toBeNull();
  },
};

/** Fleet's pause, from auto-release: the same mark, with its author in the tooltip. */
export const PausedByFleet: Story = {
  args: { said: "Paused by Fleet 4m ago: work saved on branch armada/job-2-debounce, slot released" },
};
