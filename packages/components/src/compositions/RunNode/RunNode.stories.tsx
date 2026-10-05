import type { Meta, StoryObj } from "@storybook/react-vite";
import { expect, fn } from "storybook/test";

import { RunNode } from "./RunNode";

/**
 * One node of a Job's run: a band naming its kind in its state's hue, then a
 * body of three lines. **A step card in each state a step is in**, and the
 * narrow card a gate or group takes beside the step it belongs to.
 */
const meta: Meta<typeof RunNode> = {
  title: "Compositions/Run node",
  component: RunNode,
  args: {
    kind: "step",
    name: "Implement",
    id: "implement",
    traits: [{ key: "Model", value: "sonnet" }],
    activity: "not_started",
    said: "not started",
    state: "ahead",
    onOpen: fn(),
  },
};
export default meta;

type Story = StoryObj<typeof RunNode>;

/** The gate, where nothing has run: its hue is neutral. */
export const NotStarted: Story = {};

/** Running now. */
export const Running: Story = {
  args: { activity: "running", said: "running", state: "live" },
};

/** Waiting on a person. */
export const AwaitingHuman: Story = {
  args: { activity: "awaiting_human", said: "waiting on you", state: "live" },
};

/** Done. */
export const Advanced: Story = {
  args: { activity: "advanced", said: "advanced", state: "done" },
};

/** Stopped, and failed, each its own hue. */
export const Stopped: Story = {
  args: { activity: "stopped", said: "stopped", state: "live" },
};

export const Failed: Story = {
  args: { activity: "failed", said: "failed", state: "done" },
};

/** A gate or a group beside its step, at the narrow width. */
export const Narrow: Story = {
  args: { kind: "gate", name: "Checks", id: undefined, narrow: true },
  /** The one thing it proves: it is named by its kind and state, and pressing it opens. */
  play: async ({ args, canvas, userEvent }) => {
    const node = canvas.getByRole("button", { name: "Checks, not started" });
    await expect(node).toHaveAttribute("data-narrow", "true");
    await userEvent.click(node);
    await expect(args.onOpen).toHaveBeenCalledTimes(1);
  },
};
