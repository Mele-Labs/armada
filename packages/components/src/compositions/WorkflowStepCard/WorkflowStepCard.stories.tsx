import type { Meta, StoryObj } from "@storybook/react-vite";
import { expect, fn } from "storybook/test";

import { GROUP_STATE } from "../../generated/vocabulary";
import { WorkflowStepCard } from "./WorkflowStepCard";

const meta: Meta<typeof WorkflowStepCard> = {
  title: "Compositions/Workflow step card",
  component: WorkflowStepCard,
  decorators: [
    (Story) => (
      <div style={{ padding: "var(--space-6)", background: "var(--surface-canvas)" }}>
        <Story />
      </div>
    ),
  ],
};
export default meta;

type Story = StoryObj<typeof WorkflowStepCard>;

// A step of the run, as the Workflow tab draws it (owner, 30 Sep 2026): what
// needs a person first, in its hue; then where the work has got to — the group
// bar and the line on the step at work, how long and its state on a step that
// ran, and nothing on a step nothing entered.

/** A step nothing has entered: its number, its name, and the gate's chip where a person answers. */
export const NotStarted: Story = {
  args: {
    kind: "step",
    name: "Review the change",
    activity: "not_started",
    said: "not started",
    ordinal: 4,
    gate: "will ask you",
    onOpen: fn(),
  },
};

/** The plan's four groups as the step at work's bar: two passed, one working, one to come. */
const FOUR_GROUPS = { groups: ["done", "done", "working", "open"], label: "2 of 4 groups done" } as const;

/**
 * The step the Job is on — edged, washed and swept, the live phase's own three
 * things (`design-system.md`, Motion) — with its groups as a bar and how long
 * and its Drones beside it.
 *
 * **A `play`, because a still cannot show a loop**, and the thing most at risk
 * here is the second one: one loop per card, so the mark beside the sweep has
 * to hold still. **And because a still cannot say where the count went**: it
 * is the bar's name and tooltip, never text beside the segments it counts.
 */
export const Running: Story = {
  args: {
    kind: "step",
    name: "Implement",
    activity: "running",
    said: "running",
    ordinal: 2,
    current: true,
    line: "55m · 2 Drones",
    bar: FOUR_GROUPS,
    onOpen: fn(),
  },
  // The loop is the claim; the test run otherwise emulates reduced motion.
  parameters: { motion: "on" },
  play: async ({ canvas, canvasElement }) => {
    const card = canvasElement.querySelector(".armada-wf-card")!;
    const sweep = card.querySelector(".armada-wf-card__sweep");
    await expect(sweep).not.toBeNull();
    // The bar itself is a pseudo-element, so the loop is read off the subtree.
    await expect(sweep!.getAnimations({ subtree: true }).length).toBeGreaterThan(0);
    // And nothing else on the card loops: the mark that pulses on a card with
    // no sweep holds still under one.
    await expect(card.querySelector(".armada-step-mark[data-pulsing]")).toBeNull();
    await expect(canvas.getByRole("img", { name: "2 of 4 groups done" })).toBeInTheDocument();
    // The line beside it carries how long and the Drones, and no count.
    await expect(canvas.getByText("55m · 2 Drones")).toBeVisible();
    await expect(canvas.queryByText(/groups/, { selector: ".armada-wf-card__line" })).toBeNull();
  },
};

/** The step at work, with a Check that failed at a group's boundary: the failure first, then the bar. */
export const RunningWithAFailedCheck: Story = {
  args: {
    ...Running.args,
    line: "1h 02m",
    needs: [{ says: "screens_test failed", tone: "failed" }],
    bar: { groups: ["done", "done", "failed", "open"], label: "2 of 4 groups done" },
  } as Story["args"],
};

/**
 * A step waiting on a person says so before anything else — and it is the
 * step the Job is on, which is the pair worth drawing beside the one above:
 * **a card that is not working does not move at all.**
 */
export const WaitingOnYou: Story = {
  args: {
    kind: "step",
    name: "Review the change",
    activity: "awaiting_human",
    said: "awaiting review",
    ordinal: 4,
    current: true,
    needs: [{ says: "Waiting on you", tone: "waiting" }],
    line: "12m",
    gate: "will ask you",
    onOpen: fn(),
  },
  // Under reduced motion nothing moves anyway, so holding still would prove nothing.
  parameters: { motion: "on" },
  play: async ({ canvasElement }) => {
    const card = canvasElement.querySelector(".armada-wf-card")!;
    await expect(card.querySelector(".armada-wf-card__sweep")).toBeNull();
    await expect(card.getAnimations({ subtree: true })).toHaveLength(0);
  },
};

/** A step that ran: how long it took and what it came to. */
export const Advanced: Story = {
  args: {
    kind: "step",
    name: "Plan the change",
    activity: "advanced",
    said: "advanced",
    ordinal: 1,
    line: "6m 00s · advanced",
    onOpen: fn(),
  },
};

/** A step that stopped says why first, and then only how long. */
export const Stopped: Story = {
  args: {
    kind: "step",
    name: "Implement",
    activity: "stopped",
    said: "stopped",
    ordinal: 2,
    needs: [{ says: "typecheck failed", tone: "failed" }],
    line: "48m · attempt 3",
    onOpen: fn(),
  },
};

/** A group inside the step, at its boundary's Checks. */
export const Group: Story = {
  args: {
    kind: "group",
    name: "Group 3",
    activity: "running",
    said: "checking",
    facts: [{ value: "2 tasks" }, { value: "7 checks" }],
    onOpen: fn(),
  },
};

/**
 * A group drawing its own registry row through `mark` — `GROUP_STATE`'s
 * `retrying`: its glyph, its `--status-awaiting-review` hue and its verb,
 * where the step machine's nearest activity would have drawn `--fg-muted`.
 * The list's group head reads the same row.
 */
export const GroupWithItsOwnMark: Story = {
  args: {
    kind: "group",
    name: "Group 2",
    activity: "retrying",
    mark: { icon: GROUP_STATE.retrying!.icon!, token: GROUP_STATE.retrying!.statusToken! },
    said: GROUP_STATE.retrying!.verb!,
    facts: [{ value: "3 tasks" }, { value: "run again 1 time" }],
    onOpen: fn(),
  },
  play: async ({ canvas, canvasElement }) => {
    await expect(canvas.getByRole("button", { name: "Group 2, retrying" })).toBeVisible();
    const mark = canvasElement.querySelector(".armada-wf-card__mark")!;
    await expect(mark).not.toBeNull();
    // The registry's hue, not the activity's: no `data-activity` to key one off.
    await expect(mark.hasAttribute("data-activity")).toBe(false);
    await expect((mark as HTMLElement).style.getPropertyValue("--armada-wf-card-mark")).toBe(
      "var(--status-awaiting-review)",
    );
  },
};

/** A task inside the group, narrower again. Its id is the first fact. */
export const Task: Story = {
  args: {
    kind: "task",
    name: "Draw what is running, in four lists",
    activity: "running",
    said: "working",
    facts: [{ value: "T5" }, { value: "1 file" }, { value: "14 turns" }],
    onOpen: fn(),
  },
};

/** The one being read. */
export const Open: Story = {
  args: { ...Group.args, selected: true } as Story["args"],
};

/**
 * A card with nowhere to open is not a control.
 *
 * **A `play`, because a still cannot tell a button from a span** — and the
 * stacked run draws cards a caller has given no inspector to, where a focus
 * stop that does nothing is worse than no focus stop.
 */
export const NotAControl: Story = {
  args: {
    kind: "step",
    name: "Summarise",
    activity: "not_started",
    said: "not started",
    ordinal: 2,
  },
  play: async ({ canvas }) => {
    await expect(canvas.queryByRole("button")).toBeNull();
    await expect(canvas.getByRole("group", { name: "Summarise, not started" })).toBeVisible();
  },
};
