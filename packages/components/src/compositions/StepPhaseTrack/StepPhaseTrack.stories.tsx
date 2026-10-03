import type { Meta, StoryObj } from "@storybook/react-vite";
import { expect } from "storybook/test";

import { StepPhaseTrack, type StepPhasePart } from "./StepPhaseTrack";

const meta: Meta<typeof StepPhaseTrack> = {
  title: "Compositions/Step phase track",
  component: StepPhaseTrack,
  parameters: { motion: "on" },
  decorators: [
    (Story) => (
      <div style={{ padding: "var(--space-6)", background: "var(--surface-canvas)", maxWidth: "var(--w-step-panel-min)" }}>
        <Story />
      </div>
    ),
  ],
};
export default meta;

type Story = StoryObj<typeof StepPhaseTrack>;

const part = (phase: StepPhasePart["phase"], name: string, state: StepPhasePart["state"], label: string): StepPhasePart => ({
  phase,
  name,
  state,
  label,
});

/** A Drone at work on its task; the Checks and the Judge to come. */
export const DronesNow: Story = {
  args: {
    parts: [
      part("drone", "Drones", "now", "Drone on T4"),
      part("checks", "Checks", "next", "Checks next"),
      part("judge", "Judge", "next", "Judge next"),
    ],
  },
};

/**
 * The Drones done, the gate's Checks running, the Judge to come.
 *
 * **A `play`, because only the part now moves**: one loop on the card, and it
 * is the part a person is waiting on.
 */
export const ChecksNow: Story = {
  args: {
    parts: [
      part("drone", "Drones", "done", "Drones done"),
      part("checks", "Checks", "now", "Checks running"),
      part("judge", "Judge", "next", "Judge next"),
    ],
  },
  play: async ({ canvas, canvasElement }) => {
    await expect(canvas.getByRole("img", { name: "Checks running" })).toBeVisible();
    const moving = [...canvasElement.querySelectorAll(".armada-step-track__part")].filter(
      (one) => one.getAnimations({ subtree: true }).length > 0,
    );
    await expect(moving.map((one) => one.getAttribute("aria-label"))).toEqual(["Checks running"]);
  },
};

/** The Judge reading, everything before it done. */
export const JudgeNow: Story = {
  args: {
    parts: [
      part("drone", "Drones", "done", "Drones done"),
      part("checks", "Checks", "done", "Checks done"),
      part("judge", "Judge", "now", "Judge reading"),
    ],
  },
};

/** A person's gate holding the step at its end. Their part breathes. */
export const WaitingOnYou: Story = {
  args: {
    parts: [
      part("drone", "Drones", "done", "Drones done"),
      part("checks", "Checks", "done", "Checks done"),
      part("waiting", "You", "now", "Waiting on you"),
    ],
  },
  play: async ({ canvasElement }) => {
    const moving = [...canvasElement.querySelectorAll(".armada-step-track__part")].filter(
      (one) => one.getAnimations({ subtree: true }).length > 0,
    );
    await expect(moving.map((one) => one.getAttribute("aria-label"))).toEqual(["Waiting on you"]);
  },
};
