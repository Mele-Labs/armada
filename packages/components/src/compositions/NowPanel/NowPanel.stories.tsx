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
  question: "Split the store clock out of the writer, or wrap it?",
  options: [
    { id: "split", label: "Split it out" },
    { id: "wrap", label: "Wrap it in place" },
    { id: "defer", label: "Leave it for a later Job" },
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
  { key: "r1", of: "drone", name: "Drone on Implement", line: "Edit crates/store/src/clock.rs", state: "running", onOpen: open },
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

/** A Plan decision: nothing preselected, and the answer waits for a pick. */
export const PlanQuestion: Story = {
  args: { asks: [PLAN, JUDGE_ASK, DRONE_ASK] },
  play: async ({ canvas }) => {
    const send = canvas.getByRole("button", { name: "Answer" });
    await expect(send).toBeDisabled();
    await expect(canvas.getByRole("radio", { name: "Wrap it in place" })).not.toBeChecked();
    await userEvent.click(canvas.getByRole("radio", { name: "Wrap it in place" }));
    await expect(send).toBeEnabled();
    await userEvent.click(send);
    await expect(answer).toHaveBeenCalledWith("wrap");
    await expect(canvas.getByRole("radio", { name: "Split it out" })).toBeDisabled();
    await userEvent.click(canvas.getByRole("button", { name: "Open Judge on Review the change, asks you" }));
    await expect(open).toHaveBeenCalled();
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
