import type { Meta, StoryObj } from "@storybook/react-vite";
import { expect, fn, userEvent } from "storybook/test";

import { NowPanel, type NowAsk, type NowIssue, type NowRunning } from "./NowPanel";

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
const DRONE_ASK: NowAsk = { key: "da", kind: "drone", name: "Drone on Implement", text: "Which clock does the fixture pin?", onOpen: open };
const ISSUES: NowIssue[] = [
  { key: "i1", of: "check", name: "store", text: "store: 2 failed", said: "Check failed", onOpen: open },
  { key: "i2", of: "drone", name: "Drone on Implement", text: "No output for 14m", said: "Drone stuck", onOpen: open },
];
const RUNNING: NowRunning[] = [
  {
    key: "r1",
    of: "drone",
    name: "Drone on Implement",
    line: "Edit crates/store/src/clock.rs",
    step: { id: "implement", name: "Implement" },
    tail: ["Read crates/store/src/clock.rs", "Edit crates/store/src/clock.rs"],
    state: "running",
    onOpen: open,
  },
  { key: "r2", of: "check", name: "typecheck", state: "passed", onOpen: open },
  { key: "r3", of: "check", name: "store", state: "failed", onOpen: open },
  { key: "r4", of: "check", name: "lint", state: "running", onOpen: open },
  { key: "r5", of: "judge", name: "Judge on Review the change", state: "running", onOpen: open },
];

/** Nothing to say: the head alone, no section and no sentence. */
export const Idle: Story = {
  args: { onHide: fn() },
  play: async ({ canvas, args }) => {
    await expect(canvas.queryByRole("heading", { name: "Asks you" })).toBeNull();
    await expect(canvas.queryByRole("heading", { name: "Issues" })).toBeNull();
    await expect(canvas.queryByRole("heading", { name: "Running" })).toBeNull();
    await expect(canvas.queryByRole("listitem")).toBeNull();
    await userEvent.click(canvas.getByRole("button", { name: "Hide now" }));
    await expect(args.onHide).toHaveBeenCalled();
  },
};

/** A Drone at work, its last action live, and Checks landing as they finish. */
export const Running: Story = {
  args: { running: RUNNING },
  play: async ({ canvas }) => {
    await expect(canvas.queryByRole("heading", { name: "Asks you" })).toBeNull();
    await expect(canvas.getByRole("heading", { name: "Running" })).toBeInTheDocument();
    await expect(canvas.getByText("Edit crates/store/src/clock.rs")).toBeInTheDocument();
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

/** A running row's step presses through to the host, and its output tail opens on a press. */
export const RunningStepAndOutput: Story = {
  args: { running: RUNNING, onStep: fn(), focusedStep: "implement" },
  play: async ({ canvas, args }) => {
    await expect(canvas.queryByText("Read crates/store/src/clock.rs")).toBeNull();
    const step = canvas.getByRole("button", { name: "Show Implement on the canvas" });
    await expect(step).toHaveAttribute("aria-pressed", "true");
    await userEvent.click(step);
    await expect(args.onStep).toHaveBeenCalledWith("implement");
    const fold = canvas.getByRole("button", { name: "Output of Drone on Implement" });
    await expect(fold).toHaveAttribute("aria-expanded", "false");
    await userEvent.click(fold);
    await expect(fold).toHaveAttribute("aria-expanded", "true");
    await expect(canvas.getByText(/Read crates\/store\/src\/clock.rs/)).toBeInTheDocument();
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
