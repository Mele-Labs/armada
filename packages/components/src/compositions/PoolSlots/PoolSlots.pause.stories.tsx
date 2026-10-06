import type { Meta, StoryObj } from "@storybook/react-vite";
import { expect, fn, within } from "storybook/test";

import { PoolSlots } from "./PoolSlots";
import { held, openTile, slot, tile } from "./PoolSlots.fixtures";

/** Pause and Resume on a Job's tile, and the mark beside a paused Job's state. */
const meta: Meta<typeof PoolSlots> = {
  title: "Compositions/Pool slots",
  component: PoolSlots,
  args: { onOpenJob: fn() },
};
export default meta;

type Story = StoryObj<typeof PoolSlots>;

const FILES = ["src/reader/retry.rs", "notes/retry.md"];
const RUNNING = held({ job_id: "RUN", job_title: "Debounce the Job Board", status: "running", branch: "armada/2-debounce", held: [{ why: "not_terminal", status: "running" }] });
const BAY = slot(3, { held: { state: "job", job_id: "RUN", job_title: "Debounce the Job Board" }, branch: "armada/2-debounce" });

/**
 * **Pause is sent only from its confirm**, which lists the git effects: the
 * Drone and its processes that stop, the uncommitted files committed to the
 * branch as a WIP commit, the slot released, and the step that restarts on
 * Resume. Cancel sends nothing.
 */
export const PauseAJob: Story = {
  name: "Pause a Job",
  args: {
    rows: [
      {
        name: "slot-3",
        slot: BAY,
        held: { ...RUNNING, path: BAY.path },
        offered: { clear: false, deleteBranch: false, forget: false, pause: true },
        cost: { files: FILES },
      },
    ],
    onPause: fn(),
  },
  play: async ({ args, canvas, userEvent }) => {
    const open = await openTile(canvas, userEvent, "slot-3");
    await userEvent.click(open.getByRole("button", { name: "Pause" }));
    await expect(args.onPause).not.toHaveBeenCalled();
    const confirm = within(canvas.getByRole("group", { name: "Pause slot-3" }));
    await expect(confirm.getByText("Stops the Drone and its processes")).toBeInTheDocument();
    await expect(confirm.getByText("Commits uncommitted files to branch armada/2-debounce as a WIP commit")).toBeInTheDocument();
    await expect(confirm.getByText("Releases slot-3")).toBeInTheDocument();
    await expect(confirm.getByText("The step restarts on Resume")).toBeInTheDocument();
    await userEvent.click(confirm.getByRole("button", { name: "Pause" }));
    await expect(args.onPause).toHaveBeenCalledWith("RUN");
  },
};

/** A Job parked at a gate has no process to stop, so its confirm lists the release and nothing else where no files are uncommitted. */
export const PauseAJobAtAGate: Story = {
  name: "Pause a Job at a gate",
  args: {
    rows: [
      {
        name: "slot-3",
        slot: BAY,
        held: { ...RUNNING, status: "awaiting_review", held: [], path: BAY.path },
        offered: { clear: false, deleteBranch: false, forget: false, pause: true },
        cost: { files: [] },
      },
    ],
    onPause: fn(),
  },
  play: async ({ canvas, userEvent }) => {
    const open = await openTile(canvas, userEvent, "slot-3");
    await userEvent.click(open.getByRole("button", { name: "Pause" }));
    const confirm = within(canvas.getByRole("group", { name: "Pause slot-3" }));
    await expect(confirm.getByText("Releases slot-3")).toBeInTheDocument();
    await expect(confirm.queryByText("Stops the Drone and its processes")).toBeNull();
    await expect(confirm.queryByText("The step restarts on Resume")).toBeNull();
    await expect(confirm.queryByText(/WIP commit/)).toBeNull();
  },
};

/**
 * A paused Job's tile carries the mark, and the panel offers Resume. **Resume
 * is sent at once**: it loses nothing, so there is nothing to confirm. Pause is
 * not offered on a Job that is paused.
 */
export const APausedJob: Story = {
  name: "A paused Job",
  args: {
    rows: [
      {
        name: "run",
        held: { ...RUNNING, status: "awaiting_review", held: [] },
        paused: "Paused 4 minutes ago: work saved on branch armada/2-debounce, slot released",
        offered: { clear: false, deleteBranch: false, forget: false, resume: true },
      },
    ],
    onResume: fn(),
  },
  play: async ({ args, canvas, userEvent }) => {
    const mark = "Paused 4 minutes ago: work saved on branch armada/2-debounce, slot released";
    await expect(tile(canvas, "run").getByRole("img", { name: mark })).toBeInTheDocument();
    const open = await openTile(canvas, userEvent, "run");
    await expect(open.getByRole("img", { name: mark })).toBeInTheDocument();
    await expect(open.queryByRole("button", { name: "Pause" })).toBeNull();
    await userEvent.click(open.getByRole("button", { name: "Resume" }));
    await expect(args.onResume).toHaveBeenCalledWith("RUN");
  },
};
