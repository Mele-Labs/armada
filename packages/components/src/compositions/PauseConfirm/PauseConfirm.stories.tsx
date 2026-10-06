import type { Meta, StoryObj } from "@storybook/react-vite";
import { expect, fn } from "storybook/test";
import { PauseConfirm, ResumeConfirm } from "./PauseConfirm";

const meta: Meta<typeof PauseConfirm> = {
  title: "Compositions/Pause confirm",
  component: PauseConfirm,
};
export default meta;

type Story = StoryObj<typeof PauseConfirm>;

/**
 * A running Job: its Drone and processes end, uncommitted files are committed
 * to its branch, its slot goes back, and the step restarts on Resume. Facts
 * only, and no counts.
 */
export const ARunningJob: Story = {
  args: {
    facts: { branch: "armada/job-2-debounce", files: ["src/reader/retry.rs", "notes/retry.md"], slot: "slot-3", running: true },
    onConfirm: fn(),
    onCancel: fn(),
  },
  play: async ({ args, canvas, userEvent }) => {
    await expect(canvas.getByText("Stops the Drone and its processes")).toBeInTheDocument();
    await expect(canvas.getByText("Commits uncommitted files to branch armada/job-2-debounce as a WIP commit")).toBeInTheDocument();
    await expect(canvas.getByText("Releases slot-3")).toBeInTheDocument();
    await expect(canvas.getByText("The step restarts on Resume")).toBeInTheDocument();
    // Cancel holds focus, so Enter sends nothing.
    await userEvent.keyboard("{Enter}");
    await expect(args.onConfirm).not.toHaveBeenCalled();
    await userEvent.click(canvas.getByRole("button", { name: "Pause" }));
    await expect(args.onConfirm).toHaveBeenCalled();
  },
};

/** A Job parked at a review gate has no process: the confirm says the release, and the commit only where files are there. */
export const ParkedAtAGate: Story = {
  args: { facts: { branch: "armada/job-2-debounce", files: [], slot: "slot-3", running: false }, onConfirm: fn(), onCancel: fn() },
  play: async ({ canvas }) => {
    await expect(canvas.getByText("Releases slot-3")).toBeInTheDocument();
    await expect(canvas.queryByText("Stops the Drone and its processes")).toBeNull();
    await expect(canvas.queryByText("The step restarts on Resume")).toBeNull();
    await expect(canvas.queryByText(/WIP commit/)).toBeNull();
  },
};

/** A refusal is said in the confirm, where the press was, and the dialog stays. */
export const Refused: Story = {
  args: {
    facts: { branch: "armada/job-2-debounce", files: [], slot: "slot-3", running: false },
    refused: "Not paused: its Checks are reading the worktree",
    onConfirm: fn(),
    onCancel: fn(),
  },
  play: async ({ canvas }) => {
    await expect(canvas.getByRole("alert")).toHaveTextContent("Not paused: its Checks are reading the worktree");
  },
};

/**
 * What an act on a paused Job opens. **Resume only resumes**: it says the act
 * pressed is not sent, and says nothing about a full pool, which the Job's own
 * state shows afterwards.
 */
export const ResumeAfterAnAct: StoryObj<typeof ResumeConfirm> = {
  render: (args) => <ResumeConfirm {...args} />,
  args: { branch: "armada/job-2-debounce", pressed: true, onConfirm: fn(), onCancel: fn() },
  play: async ({ args, canvas, userEvent }) => {
    await expect(canvas.getByText("Puts branch armada/job-2-debounce back in a slot")).toBeInTheDocument();
    await expect(canvas.getByText("Does not send the act you pressed")).toBeInTheDocument();
    await expect(canvas.queryByText(/slot is free|waiting|full/i)).toBeNull();
    await userEvent.click(canvas.getByRole("button", { name: "Cancel" }));
    await expect(args.onCancel).toHaveBeenCalled();
    await expect(args.onConfirm).not.toHaveBeenCalled();
  },
};

/** Resume asked for by name draws no line about an act that was never pressed. */
export const ResumeByName: StoryObj<typeof ResumeConfirm> = {
  render: (args) => <ResumeConfirm {...args} />,
  args: { branch: "armada/job-2-debounce", pressed: false, onConfirm: fn(), onCancel: fn() },
  play: async ({ canvas }) => {
    await expect(canvas.queryByText("Does not send the act you pressed")).toBeNull();
  },
};
