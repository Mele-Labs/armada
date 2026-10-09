import type { Meta, StoryObj } from "@storybook/react-vite";
import { expect, fn, userEvent, waitFor } from "storybook/test";

import { NowPanel, type NowAct, type NowAsk, type NowSketch, type NowIssue, type NowRunning, type NowWaiting } from "./NowPanel";

/** What a Job is doing and what it wants, beside its canvas. A section is drawn only when it holds a row. */
const meta: Meta<typeof NowPanel> = {
  title: "Compositions/Now panel",
  component: NowPanel,
  decorators: [
    (Story) => (
      <div style={{ display: "flex" }}>
        <Story />
      </div>
    ),
  ],
};
export default meta;

type Story = StoryObj<typeof NowPanel>;

const open = fn();
const answer = fn();
const act = fn();
const CHECK_ACTS: NowAct[] = [
  { key: "retry", glyph: "retry", said: "Retry now", onAct: act },
  { key: "skip", glyph: "skip", said: "Skip check", onAct: act },
];

const PLAN: NowAsk = {
  key: "plan",
  kind: "plan",
  decisions: [
    {
      id: "shape",
      question: "Split the store clock out of the writer, or wrap it?",
      options: [
        { id: "split", label: "Split it out" },
        { id: "wrap", label: "Wrap it in place" },
      ],
    },
    {
      id: "tests",
      question: "Pin the clock in the fixtures, or add a fake?",
      options: [
        { id: "pin", label: "Pin it in the fixtures" },
        { id: "fake", label: "Add a fake clock" },
      ],
    },
  ],
  onAnswer: answer,
};
const JUDGE_ASK: NowAsk = { key: "ja", kind: "judge", name: "Judge on Review the change", text: "Is the retry cap in scope?", onOpen: open };
const DRONE_ASK: NowAsk = { key: "da", kind: "drone", name: "Implement Drone", text: "Which clock does the fixture pin?", onOpen: open };
const ISSUES: NowIssue[] = [
  { key: "i1", of: "check", name: "store", text: "store: 2 failed", said: "Check failed", acts: CHECK_ACTS, onOpen: open },
  {
    key: "i2",
    of: "drone",
    name: "Implement Drone",
    text: "No output for 14m",
    said: "Drone stuck",
    acts: [
      { key: "redirect", glyph: "redirect", said: "Redirect Drone", onAct: act },
      { key: "restart", glyph: "retry_step", said: "Retry step", onAct: act },
    ],
    onOpen: open,
  },
];
const RUNNING: NowRunning[] = [
  {
    key: "r1",
    of: "drone",
    name: "Implement Drone",
    line: "Edit crates/store/src/clock.rs",
    step: { id: "implement", name: "Implement" },
    tail: ["Read crates/store/src/clock.rs", "Edit crates/store/src/clock.rs"],
    state: "running",
    onOpen: open,
  },
  { key: "r2", of: "check", name: "typecheck", state: "passed", onOpen: open },
  { key: "r3", of: "check", name: "store", state: "failed", acts: CHECK_ACTS, onOpen: open },
  { key: "r4", of: "check", name: "lint", state: "running", acts: CHECK_ACTS, onOpen: open },
  { key: "r5", of: "judge", name: "Judge on Review the change", state: "running", onOpen: open },
];

/** Nothing to say and no reason given: the sentence the owner reads as a defect, and no section. */
export const Idle: Story = {
  args: { onHide: fn() },
  play: async ({ canvas, args }) => {
    await expect(canvas.getByText("Nothing is actively running on this job")).toBeInTheDocument();
    await expect(canvas.queryByRole("heading", { name: "Asks you" })).toBeNull();
    await expect(canvas.queryByRole("heading", { name: "Issues" })).toBeNull();
    await expect(canvas.queryByRole("heading", { name: "Running" })).toBeNull();
    await expect(canvas.queryByRole("listitem")).toBeNull();
    await userEvent.click(canvas.getByRole("button", { name: "Hide now" }));
    await expect(args.onHide).toHaveBeenCalled();
  },
};

/** A reason for each way a Job runs nothing: a resource, another Job, a transition, a one-off step. */
export const Waiting: Story = {
  args: {
    waiting: [
      { key: "w1", kind: "resource", text: "Worktree slot" },
      { key: "w2", kind: "job", text: "Pin the store clock", onOpen: open },
      { key: "w3", kind: "transition", text: "Plan to Implement" },
      { key: "w4", kind: "step", text: "Handoff, a one-off step" },
    ] satisfies NowWaiting[],
  },
  play: async ({ canvas }) => {
    await expect(canvas.queryByText("Nothing is actively running on this job")).toBeNull();
    await expect(canvas.getByRole("heading", { name: "Waiting" })).toBeInTheDocument();
    await expect(canvas.getByRole("img", { name: "Resource" })).toBeInTheDocument();
    await expect(canvas.getByText("Worktree slot")).toBeInTheDocument();
    await expect(canvas.getByText("Plan to Implement")).toBeInTheDocument();
    await expect(canvas.queryByRole("button", { name: /Resource/ })).toBeNull();
    await userEvent.click(canvas.getByRole("button", { name: "Open Job Pin the store clock" }));
    await expect(open).toHaveBeenCalled();
  },
};

/** Checks retry or skip from their row, every Check skips from the band, and issues carry their own fixes. */
export const QuickActs: Story = {
  args: { running: RUNNING, issues: ISSUES, onSkipAll: fn() },
  play: async ({ canvas, args }) => {
    await userEvent.click(canvas.getAllByRole("button", { name: "Retry now" })[0]!);
    await userEvent.click(canvas.getAllByRole("button", { name: "Skip check" })[0]!);
    await expect(act).toHaveBeenCalledTimes(2);
    await userEvent.click(canvas.getByRole("button", { name: "Skip all checks" }));
    await expect(args.onSkipAll).toHaveBeenCalled();
    await expect(canvas.getByRole("button", { name: "Redirect Drone" })).toBeInTheDocument();
    await expect(canvas.getByRole("button", { name: "Retry step" })).toBeInTheDocument();
  },
};

/** A Drone at work, its last action live, and Checks landing as they finish. */
export const Running: Story = {
  args: { running: RUNNING },
  play: async ({ canvas }) => {
    await expect(canvas.queryByRole("heading", { name: "Asks you" })).toBeNull();
    await expect(canvas.getByRole("heading", { name: "Running" })).toBeInTheDocument();
    await expect(canvas.queryByText("Drones")).toBeNull();
    await expect(canvas.getAllByRole("img", { name: "Drone" }).length).toBeGreaterThan(0);
    await expect(canvas.getAllByRole("img", { name: "Check" }).length).toBeGreaterThan(0);
    await expect(canvas.getAllByRole("img", { name: "Judge" }).length).toBeGreaterThan(0);
    await expect(canvas.getByText("Edit crates/store/src/clock.rs", { selector: "span" })).toBeInTheDocument();
    await expect(canvas.getByRole("button", { name: "Open typecheck, passed" })).toBeInTheDocument();
    await expect(canvas.getByRole("button", { name: "Open store, failed" })).toBeInTheDocument();
    await userEvent.click(canvas.getByRole("button", { name: "Open Judge on Review the change, running" }));
    await expect(open).toHaveBeenCalled();
  },
};

/** Decisions one at a time: nothing preselected, Next waits for a pick, and the last carries Answer. */
export const PlanQuestion: Story = {
  args: { asks: [PLAN, JUDGE_ASK, DRONE_ASK] },
  play: async ({ canvas }) => {
    await expect(canvas.queryByText("Plan question")).toBeNull();
    await expect(canvas.getByRole("img", { name: "Judge" })).toBeInTheDocument();
    await expect(canvas.getByRole("img", { name: "Drone" })).toBeInTheDocument();
    await expect(canvas.queryByRole("button", { name: "Answer" })).toBeNull();
    const next = canvas.getByRole("button", { name: "Next" });
    await expect(next).toBeDisabled();
    await expect(canvas.getByRole("radio", { name: "Wrap it in place" })).not.toBeChecked();
    await expect(canvas.queryByRole("radio", { name: "Add a fake clock" })).toBeNull();
    await expect(canvas.getByRole("img", { name: "Split the store clock out of the writer, or wrap it?" })).toHaveAttribute("aria-current", "step");
    await userEvent.click(canvas.getByRole("radio", { name: "Wrap it in place" }));
    await userEvent.click(next);
    await expect(canvas.queryByRole("radio", { name: "Wrap it in place" })).toBeNull();
    await expect(canvas.getByRole("radio", { name: "Add a fake clock" })).not.toBeChecked();
    await expect(canvas.queryByRole("button", { name: "Next" })).toBeNull();
    const send = canvas.getByRole("button", { name: "Answer" });
    await expect(send).toBeDisabled();
    await userEvent.click(canvas.getByRole("radio", { name: "Add a fake clock" }));
    await userEvent.click(send);
    await expect(answer).toHaveBeenCalledWith({ shape: "wrap", tests: "fake" });
    await expect(canvas.getByRole("radio", { name: "Add a fake clock" })).toBeDisabled();
    await userEvent.click(canvas.getByRole("button", { name: "Open Judge on Review the change, asks you" }));
    await expect(open).toHaveBeenCalled();
  },
};

/** One Drone at work: its output is open already. A second Drone folds both, and a press opens one. */
export const RunningStepAndOutput: Story = {
  args: { running: RUNNING, onStep: fn(), focusedStep: "implement" },
  play: async ({ canvas, args }) => {
    await expect(canvas.getByText(/Read crates\/store\/src\/clock.rs/)).toBeInTheDocument();
    const fold = canvas.getByRole("button", { name: "Output of Implement Drone" });
    await expect(fold).toHaveAttribute("aria-expanded", "true");
    await userEvent.click(fold);
    await expect(fold).toHaveAttribute("aria-expanded", "false");
    await expect(canvas.queryByText(/Read crates\/store\/src\/clock.rs/)).toBeNull();
    const step = canvas.getByRole("button", { name: "Show Implement on the canvas" });
    await expect(step).toHaveAttribute("aria-pressed", "true");
    await userEvent.click(step);
    await expect(args.onStep).toHaveBeenCalledWith("implement");
  },
};

/** Two Drones at work: neither opens its output until pressed. */
export const TwoDrones: Story = {
  args: {
    running: [
      RUNNING[0]!,
      { key: "r6", of: "drone", name: "Plan Drone", step: { id: "plan", name: "Plan" }, tail: ["Read docs/scope.md"], state: "running", onOpen: open },
    ],
  },
  play: async ({ canvas }) => {
    await expect(canvas.getByRole("button", { name: "Output of Plan Drone" })).toHaveAttribute("aria-expanded", "false");
    await expect(canvas.queryByText("Read docs/scope.md")).toBeNull();
  },
};

/** A failed Check and a stuck Drone. */
export const Issues: Story = {
  args: { issues: ISSUES },
  play: async ({ canvas }) => {
    await expect(canvas.getByRole("heading", { name: "Issues" })).toBeInTheDocument();
    await expect(canvas.getByRole("img", { name: "Check failed" })).toBeInTheDocument();
    await expect(canvas.getByRole("img", { name: "Drone stuck" })).toBeInTheDocument();
  },
};

export const Everything: Story = {
  args: { asks: [PLAN, JUDGE_ASK], issues: ISSUES, running: RUNNING, onHide: fn() },
};

const scene = (title: string): NowSketch => ({ scene: { nodes: [{ id: "a", kind: "box", x: 0, y: 0, title }], edges: [] } });
const NOW = scene("Now");
const SPLIT = scene("Split");
const WRAP = scene("Wrap");
const OTHER = scene("Tests");

const SKETCHED: NowAsk = {
  key: "plan",
  kind: "plan",
  decisions: [
    {
      id: "shape",
      question: "Split the store clock out of the writer, or wrap it?",
      sketch: NOW,
      options: [
        { id: "split", label: "Split it out", sketch: SPLIT },
        { id: "wrap", label: "Wrap it in place", sketch: WRAP },
      ],
    },
    { id: "tests", question: "Pin the clock in the fixtures, or add a fake?", sketch: OTHER, options: [{ id: "pin", label: "Pin it in the fixtures" }] },
    { id: "scope", question: "Take the retry cap in this Job?", options: [{ id: "in", label: "Take it here" }] },
  ],
  onAnswer: answer,
};

/**
 * An option previews its own sketch, read against the decision's, while hovered or focused and keeps
 * it once picked; with none of those the decision's own sketch stands on its own.
 */
export const OptionPreviewsItsSketch: Story = {
  args: { asks: [SKETCHED], onSketch: fn() },
  play: async ({ canvas, args }) => {
    await expect(args.onSketch).toHaveBeenLastCalledWith({ sketch: NOW });
    const split = canvas.getByText("Split it out");
    const wrap = canvas.getByText("Wrap it in place");
    await userEvent.hover(split);
    await expect(args.onSketch).toHaveBeenLastCalledWith({ sketch: SPLIT, against: NOW });
    await userEvent.hover(wrap);
    await expect(args.onSketch).toHaveBeenLastCalledWith({ sketch: WRAP, against: NOW });
    await userEvent.unhover(wrap);
    await expect(args.onSketch).toHaveBeenLastCalledWith({ sketch: NOW });
    await userEvent.click(split);
    await userEvent.unhover(split);
    await expect(args.onSketch).toHaveBeenLastCalledWith({ sketch: SPLIT, against: NOW });
  },
};

/** Keyboard focus previews as hover does. */
export const FocusPreviewsItsSketch: Story = {
  args: { asks: [SKETCHED], onSketch: fn() },
  play: async ({ canvas, args }) => {
    canvas.getByRole("radio", { name: "Wrap it in place" }).focus();
    await waitFor(() => expect(args.onSketch).toHaveBeenLastCalledWith({ sketch: WRAP, against: NOW }));
    canvas.getByRole("radio", { name: "Wrap it in place" }).blur();
    await waitFor(() => expect(args.onSketch).toHaveBeenLastCalledWith({ sketch: NOW }));
  },
};

/** Next moves to the next decision's sketch, and a decision with none reports none. */
export const PlanWithSketches: Story = {
  args: { asks: [SKETCHED], onSketch: fn() },
  play: async ({ canvas, args }) => {
    await userEvent.click(canvas.getByText("Split it out"));
    await userEvent.click(canvas.getByRole("button", { name: "Next" }));
    await expect(args.onSketch).toHaveBeenLastCalledWith({ sketch: OTHER });
    await userEvent.click(canvas.getByText("Pin it in the fixtures"));
    await userEvent.click(canvas.getByRole("button", { name: "Next" }));
    await expect(args.onSketch).toHaveBeenLastCalledWith(undefined);
  },
};

/** A Judge's question with a sketch draws the Sketch and Canvas switch; an ask without one draws none. */
export const JudgeWithSketch: Story = {
  args: { asks: [{ key: "ja", kind: "judge", name: "Judge on Review the change", text: "Is the retry cap in scope?", onOpen: open, sketch: NOW }], onSketch: fn(), onSketchView: fn(), sketchView: "sketch" },
  play: async ({ canvas, args }) => {
    await expect(args.onSketch).toHaveBeenLastCalledWith({ sketch: NOW });
    await expect(canvas.getByRole("button", { name: "Sketch" })).toHaveAttribute("aria-pressed", "true");
    await userEvent.click(canvas.getByRole("button", { name: "Canvas" }));
    await expect(args.onSketchView).toHaveBeenCalledWith("canvas");
  },
};

export const AskWithoutSketch: Story = {
  args: { asks: [DRONE_ASK], onSketch: fn(), onSketchView: fn() },
  play: async ({ canvas, args }) => {
    await expect(args.onSketch).toHaveBeenLastCalledWith(undefined);
    await expect(canvas.queryByRole("button", { name: "Sketch" })).toBeNull();
  },
};

/** What was asked about a part of a sketch sits under the ask. */
export const RemarksUnderTheAsk: Story = {
  args: { asks: [SKETCHED], thread: [{ key: "r1", about: "Writer", said: "Does it still flush on close?" }] },
  play: async ({ canvas }) => {
    await expect(canvas.getByText("Does it still flush on close?")).toBeInTheDocument();
  },
};
