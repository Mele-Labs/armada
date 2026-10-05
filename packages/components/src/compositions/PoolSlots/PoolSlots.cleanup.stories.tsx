import type { Meta, StoryObj } from "@storybook/react-vite";
import { expect, fn, within } from "storybook/test";
import type { WorktreeHeld } from "@armada/protocol";

import { PoolSlots, type TileRow } from "./PoolSlots";
import { held, openTile, panel, slot, tile } from "./PoolSlots.fixtures";

/** A Job's worktree, in the grid beside the bays and in the panel that gives it back. */
const meta: Meta<typeof PoolSlots> = {
  title: "Compositions/Pool slots/Cleanup",
  component: PoolSlots,
  args: { onOpenJob: fn() },
};
export default meta;

type Story = StoryObj<typeof PoolSlots>;

const UNMERGED = { why: "unmerged", base: "main", commits: 3, tip: "9f1c2ab84d5e" } as const;

/** A worktree outside the pool, as the screen builds its tile. */
function outside(over: Partial<WorktreeHeld>, rest: Partial<TileRow> = {}): TileRow {
  const row = held(over);
  return { name: row.job_id.toLowerCase(), held: row, sat: "4 days", ...rest };
}

const CLEAR = { clear: true, deleteBranch: false, forget: false };
const BRANCH = { clear: false, deleteBranch: true, forget: false };

/**
 * A Job's worktree outside the pool is a tile after the bays, in the same
 * grid: its Job as a link, its branch, and how long it has sat. **Its state is
 * the reason it is held**: held while the Job runs, kept for what it holds,
 * free where nothing holds it, gone where only the branch or record is left.
 */
export const OutsideTiles: Story = {
  name: "Outside the pool",
  args: {
    rows: [
      { slot: slot(1, { held: { state: "free" } }) },
      outside({ job_id: "RUN", status: "running", held: [{ why: "not_terminal", status: "running" }] }),
      outside({ job_id: "KEPT", held: [{ why: "uncommitted", files: ["src/a.rs"] }] }),
      outside({ job_id: "FREE" }),
      outside({ job_id: "GONE", on_disk: false, held: [UNMERGED] }),
      outside({ job_id: "RECORD", on_disk: false }),
    ],
    onAct: fn(),
  },
  play: async ({ args, canvas, userEvent }) => {
    const tiles = within(canvas.getByRole("list", { name: "Worktree slots" })).getAllByRole("listitem");
    // The pool's own add tile closes the bays, and the worktrees follow it.
    await expect(tiles.map((one) => one.getAttribute("aria-label"))).toEqual(["slot-1", null, "run", "kept", "free", "gone", "record"]);
    await expect(tile(canvas, "run").getByRole("img", { name: "Held: the Job has not ended" })).toBeInTheDocument();
    await expect(tile(canvas, "kept").getByRole("img", { name: "Kept: uncommitted" })).toBeInTheDocument();
    await expect(tile(canvas, "free").getByRole("img", { name: "Free: nothing holds it" })).toBeInTheDocument();
    await expect(tile(canvas, "gone").getByRole("img", { name: "Kept: unmerged" })).toBeInTheDocument();
    await expect(tile(canvas, "record").getByRole("img", { name: "Checkout gone" })).toBeInTheDocument();
    await expect(tile(canvas, "kept").getByLabelText("Last moved 4 days ago")).toHaveTextContent("4 days");
    await userEvent.click(tile(canvas, "kept").getByRole("button", { name: "Port the settings selectors" }));
    await expect(args.onOpenJob).toHaveBeenCalledWith("KEPT");
    // The Job link is a link and not a way into the panel.
    await expect(canvas.queryByRole("dialog")).toBeNull();
  },
};

/**
 * The panel lists every reason a worktree is held as a short row, a mark and a
 * word, with what it carries under it. **Uncommitted names the files and how
 * long they have sat**; Unmerged says its commits and the tip. A worktree that
 * holds nothing draws no rows and no heading.
 */
export const WhatItHolds: Story = {
  name: "What it holds",
  args: {
    rows: [
      outside(
        {
          job_id: "ALL",
          held: [
            { why: "uncommitted", files: ["src/a.rs", "notes/b.md"] },
            UNMERGED,
            { why: "locked", reason: "claimed by a Pilot" },
            { why: "depended_on", by: ["job-handle"] },
            { why: "base_unanswered", detail: "origin/main not found" },
            { why: "unreadable", detail: "git status failed" },
          ],
        },
        { offered: CLEAR },
      ),
      outside({ job_id: "NOTHING" }, { offered: CLEAR }),
      outside({ job_id: "RUN", status: "running", held: [{ why: "not_terminal", status: "running" }] }),
    ],
  },
  play: async ({ canvas, userEvent }) => {
    const all = await openTile(canvas, userEvent, "all");
    const holds = within(all.getByRole("region", { name: "What it holds" }));
    await expect(holds.getAllByRole("listitem").filter((one) => one.className.includes("armada-holds__row"))).toHaveLength(6);
    await expect(holds.getByLabelText("Uncommitted files")).toHaveTextContent("src/a.rs");
    await expect(holds.getByLabelText("Uncommitted files")).toHaveTextContent("notes/b.md");
    await expect(holds.getByLabelText("Last moved 4 days ago")).toBeInTheDocument();
    await expect(holds.getByLabelText("3 commits not on main")).toHaveTextContent("3 commits");
    await expect(holds.getByLabelText("Tip 9f1c2ab84d5e")).toBeInTheDocument();
    await expect(holds.getByText("claimed by a Pilot")).toBeInTheDocument();
    await expect(holds.getByLabelText("Depended on by")).toHaveTextContent("job-handle");
    await expect(holds.getByText("origin/main not found")).toBeInTheDocument();
    await expect(holds.getByText("git status failed")).toBeInTheDocument();
    await userEvent.keyboard("{Escape}");

    const nothing = await openTile(canvas, userEvent, "nothing");
    await expect(nothing.queryByRole("region", { name: "What it holds" })).toBeNull();
    await userEvent.keyboard("{Escape}");

    const running = await openTile(canvas, userEvent, "run");
    await expect(running.getByRole("region", { name: "What it holds" })).toHaveTextContent(/running/);
  },
};

/**
 * **Clear is sent only from its confirm**, which names the files it destroys
 * with how long they have sat, the checkout it removes, and the branch it
 * leaves standing at its tip. Cancel sends nothing.
 */
export const ClearConfirm: Story = {
  name: "Clear confirm",
  args: {
    rows: [
      outside(
        { job_id: "DIRTY", held: [{ why: "uncommitted", files: ["src/a.rs", "notes/b.md"] }, UNMERGED] },
        { offered: CLEAR, cost: { files: ["src/a.rs", "notes/b.md"], branch: { name: "armada/01JOB0001", commits: 3, tip: "9f1c2ab84d5e" } } },
      ),
    ],
    onClear: fn(),
  },
  play: async ({ args, canvas, userEvent }) => {
    const open = await openTile(canvas, userEvent, "dirty");
    await userEvent.click(open.getByRole("button", { name: "Clear" }));
    await expect(args.onClear).not.toHaveBeenCalled();

    const confirm = within(canvas.getByRole("group", { name: "Clear dirty" }));
    await expect(confirm.getByRole("list", { name: "Destroyed files" })).toHaveTextContent("src/a.rs");
    await expect(confirm.getByRole("list", { name: "Destroyed files" })).toHaveTextContent("notes/b.md");
    await expect(confirm.getByLabelText("Last moved 4 days ago")).toHaveTextContent("4 days");
    await expect(confirm.getByRole("list", { name: "Checkout" })).toHaveTextContent("/.armada/worktrees/01JOB0001");
    await expect(confirm.getByRole("list", { name: "Branch" })).toHaveTextContent("armada/01JOB0001 · 3 commits · 9f1c2ab84d5e");
    // Nothing else can be pressed while a confirm is up: the acts wait.
    await expect(within(open.getByRole("group", { name: "Acts" })).getByRole("button", { name: "Clear" })).toHaveAttribute(
      "aria-disabled",
      "true",
    );

    await userEvent.click(confirm.getByRole("button", { name: "Cancel" }));
    await expect(canvas.queryByRole("group", { name: "Clear dirty" })).toBeNull();
    await expect(args.onClear).not.toHaveBeenCalled();

    await userEvent.click(panel(canvas, "dirty").getByRole("button", { name: "Clear" }));
    await userEvent.click(within(canvas.getByRole("group", { name: "Clear dirty" })).getByRole("button", { name: "Clear" }));
    await expect(args.onClear).toHaveBeenCalledWith("DIRTY");
    await expect(canvas.queryByRole("group", { name: "Clear dirty" })).toBeNull();
  },
};

/** A checkout holding no uncommitted file still confirms, and names nothing destroyed. */
export const ClearConfirmDestroysNothing: Story = {
  name: "Clear confirm, nothing destroyed",
  args: {
    rows: [outside({ job_id: "CLEAN" }, { offered: CLEAR, cost: { files: [] } })],
    onClear: fn(),
  },
  play: async ({ canvas, userEvent }) => {
    const open = await openTile(canvas, userEvent, "clean");
    await userEvent.click(open.getByRole("button", { name: "Clear" }));
    const confirm = within(canvas.getByRole("group", { name: "Clear clean" }));
    await expect(confirm.queryByRole("list", { name: "Destroyed files" })).toBeNull();
    await expect(confirm.queryByRole("list", { name: "Branch" })).toBeNull();
    await expect(confirm.getByRole("list", { name: "Checkout" })).toBeInTheDocument();
  },
};

/** Delete branch names the branch and the tip it is reachable from, and sends that tip. */
export const DeleteBranchConfirm: Story = {
  name: "Delete branch confirm",
  args: {
    rows: [
      outside(
        { job_id: "GONE", on_disk: false, held: [UNMERGED] },
        { offered: BRANCH, cost: { files: [], branch: { name: "armada/01JOB0001", commits: 3, tip: "9f1c2ab84d5e" } } },
      ),
    ],
    onDeleteBranch: fn(),
  },
  play: async ({ args, canvas, userEvent }) => {
    const open = await openTile(canvas, userEvent, "gone");
    await expect(open.queryByRole("button", { name: "Clear" })).toBeNull();
    await userEvent.click(open.getByRole("button", { name: "Delete branch" }));
    const confirm = within(canvas.getByRole("group", { name: "Delete branch gone" }));
    await expect(confirm.getByRole("list", { name: "Branch" })).toHaveTextContent("armada/01JOB0001 · 3 commits · 9f1c2ab84d5e");
    await expect(confirm.getByRole("list", { name: "Tip" })).toHaveTextContent("9f1c2ab84d5e");
    await expect(args.onDeleteBranch).not.toHaveBeenCalled();
    await userEvent.click(confirm.getByRole("button", { name: "Delete branch" }));
    await expect(args.onDeleteBranch).toHaveBeenCalledWith("GONE", "9f1c2ab84d5e");
  },
};

/** Forget Job names the Job, and is sent only from its confirm. */
export const ForgetConfirm: Story = {
  name: "Forget confirm",
  args: {
    rows: [outside({ job_id: "OLD", on_disk: false }, { offered: { clear: false, deleteBranch: false, forget: true } })],
    onForget: fn(),
  },
  play: async ({ args, canvas, userEvent }) => {
    const open = await openTile(canvas, userEvent, "old");
    await userEvent.click(open.getByRole("button", { name: "Forget Job" }));
    const confirm = within(canvas.getByRole("group", { name: "Forget Job old" }));
    await expect(confirm.getByRole("list", { name: "Job" })).toHaveTextContent("Port the settings selectors");
    await expect(args.onForget).not.toHaveBeenCalled();
    await userEvent.click(confirm.getByRole("button", { name: "Forget Job" }));
    await expect(args.onForget).toHaveBeenCalledWith("OLD");
  },
};

/**
 * Acts are offered only where they apply. **A Job that has not ended offers
 * none of the three**, a worktree whose checkout is gone offers no Clear, and a
 * pool drawn with none of the reclaim wiring offers none either.
 */
export const OfferedWhereTheyApply: Story = {
  name: "Offered where they apply",
  args: {
    rows: [
      outside({ job_id: "RUN", held: [{ why: "not_terminal", status: "running" }] }),
      outside({ job_id: "KEPT", held: [UNMERGED] }, { offered: { clear: true, deleteBranch: true, forget: false } }),
      outside({ job_id: "GONE", on_disk: false }, { offered: { clear: false, deleteBranch: false, forget: true } }),
    ],
    onClear: fn(),
    onDeleteBranch: fn(),
    onForget: fn(),
  },
  play: async ({ canvas, userEvent }) => {
    const acts = ["Clear", "Delete branch", "Forget Job"];
    const offered = async (name: string) => {
      const open = await openTile(canvas, userEvent, name);
      const got = acts.filter((act) => open.queryByRole("button", { name: act }) !== null);
      await userEvent.keyboard("{Escape}");
      return got;
    };
    await expect(await offered("run")).toEqual([]);
    await expect(await offered("kept")).toEqual(["Clear", "Delete branch"]);
    await expect(await offered("gone")).toEqual(["Forget Job"]);
  },
};

/** What Clear did and what Fleet refused are said in the panel, a fact to a line, and never on the tile. */
export const ReceiptAndRefusal: Story = {
  name: "Receipt and refusal",
  args: {
    rows: [
      outside({ job_id: "DONE" }, { offered: CLEAR, receipt: ["Checkout gone", "Branch kept, 3 commits"] }),
      outside({ job_id: "NO" }, { offered: CLEAR, refused: "Not cleared: the Job has not ended" }),
    ],
  },
  play: async ({ canvas, userEvent }) => {
    await expect(canvas.queryByRole("status")).toBeNull();
    await expect(canvas.queryByRole("alert")).toBeNull();
    const done = await openTile(canvas, userEvent, "done");
    await expect(done.getByRole("status")).toHaveTextContent("Checkout gone");
    await expect(done.getByRole("status")).toHaveTextContent("Branch kept, 3 commits");
    await userEvent.keyboard("{Escape}");
    const no = await openTile(canvas, userEvent, "no");
    await expect(no.getByRole("alert")).toHaveTextContent("Not cleared: the Job has not ended");
  },
};

/** A held Job's bay carries its slot acts and its reclaim acts in one panel. */
export const BayWithItsReclaim: Story = {
  name: "A bay with its reclaim",
  args: {
    rows: [
      {
        slot: slot(2, { held: { state: "job", job_id: "01JOB0001", job_title: "Port the settings selectors", job_status: "completed_success" }, branch: "armada/01JOB0001" }),
        held: held({ held: [{ why: "uncommitted", files: ["src/a.rs"] }] }),
        sat: "4 days",
        offered: CLEAR,
        cost: { files: ["src/a.rs"] },
      },
    ],
    onAct: fn(),
    onClear: fn(),
  },
  play: async ({ canvas, userEvent }) => {
    const open = await openTile(canvas, userEvent, "slot-2");
    await expect(open.getByRole("button", { name: "Clear" })).toBeInTheDocument();
    await expect(open.getByRole("button", { name: "Close" })).toBeInTheDocument();
    await expect(open.getByRole("region", { name: "What it holds" })).toHaveTextContent("src/a.rs");
    await expect(open.getByRole("button", { name: "Port the settings selectors" })).toBeInTheDocument();
  },
};
