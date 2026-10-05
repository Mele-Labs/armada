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

/**
 * A gate stage is a capsule on the spine, not a card: one line, the kind's
 * glyph, then marks. State is the border and the glyphs' colour.
 */
const gate: Story["args"] = { kind: "gate", name: "Checks", id: undefined, traits: [], meta: [] };

/** Checks running: a failed command named inline, the live one's last line under the capsule. */
export const ChecksRunning: Story = {
  args: {
    ...gate,
    activity: "running",
    said: "running",
    state: "live",
    gate: {
      kind: "checks",
      commands: [
        { name: "build", outcome: "passed" },
        { name: "typecheck", outcome: "failed" },
        { name: "test", outcome: "running" },
        { name: "format", outcome: "waiting" },
      ],
      elapsed: "4m 3s",
      output: "test screens::approval_life::marks_a_gate ... ok",
    },
  },
};

/** Checks passed: a thin, quiet capsule. */
export const ChecksPassed: Story = {
  args: {
    ...gate,
    activity: "advanced",
    said: "advanced",
    state: "done",
    gate: { kind: "checks", commands: [{ name: "build", outcome: "passed" }, { name: "test", outcome: "passed" }] },
  },
};

/** A Judge part-way through a panel of three. */
export const JudgePartWay: Story = {
  args: {
    ...gate,
    name: "Judge",
    activity: "running",
    said: "running",
    state: "live",
    gate: { kind: "judge", panel: ["met", "met", "judging"] },
  },
};

/** A refusal: the first line beside the marks, on a flat tint. */
export const JudgeRefused: Story = {
  args: {
    ...gate,
    name: "Judge",
    activity: "failed",
    said: "failed",
    state: "ahead",
    gate: { kind: "judge", panel: ["met", "not_met", "met"], refusal: "The change adds a second retry loop" },
  },
};

/** A human gate, held, and how long it has waited. */
export const YouHeld: Story = {
  args: {
    ...gate,
    name: "You",
    activity: "awaiting_human",
    said: "needs review",
    state: "ahead",
    gate: { kind: "you", asking: "Review the change", waited: "23m 3s" },
  },
};

/** At the gate, nothing run yet: the setup. */
export const GateSetUp: Story = {
  args: {
    ...gate,
    name: "You",
    gate: { kind: "you", asking: "I review it" },
  },
  play: async ({ args, canvas, userEvent }) => {
    const node = canvas.getByRole("button", { name: "You, not started" });
    await userEvent.click(node);
    await expect(args.onOpen).toHaveBeenCalledTimes(1);
  },
};
