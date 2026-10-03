import type { Meta, StoryObj } from "@storybook/react-vite";
import { expect } from "storybook/test";

import { StepPhaseMark } from "./StepPhaseMark";

const meta: Meta<typeof StepPhaseMark> = {
  title: "Compositions/Step phase mark",
  component: StepPhaseMark,
  parameters: { motion: "on" },
};
export default meta;

type Story = StoryObj<typeof StepPhaseMark>;

/** A Drone on the step, on a task. */
export const DroneWorking: Story = { args: { phase: "drone", label: "Drone on T4" } };

/**
 * The gate running the step's Checks.
 *
 * **A `play`, because the mark has no words of its own.** Its name is the only
 * channel a reader without the glyph has, and it is the tooltip's too.
 */
export const ChecksRunning: Story = {
  args: { phase: "checks", label: "Checks running" },
  play: async ({ canvas }) => {
    await expect(canvas.getByRole("img", { name: "Checks running" })).toBeVisible();
  },
};

/** The Judge's call out on the step. */
export const JudgeReading: Story = { args: { phase: "judge", label: "Judge reading" } };

/** A question or a command on the step, waiting on a person. Holds still. */
export const WaitingOnYou: Story = { args: { phase: "waiting", label: "Waiting on you" } };
