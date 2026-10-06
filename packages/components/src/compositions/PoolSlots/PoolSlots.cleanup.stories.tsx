import type { Meta, StoryObj } from "@storybook/react-vite";
import { expect, fn, within } from "storybook/test";
import type { WorktreeHeld } from "@armada/protocol";

import { PoolSlots, type TileRow } from "./PoolSlots";
import { FOUND, STRANDED, held, openTile, panel, slot, tile } from "./PoolSlots.fixtures";

/** A Job's worktree, in the grid beside the bays and in the panel that gives it back. */
const meta: Meta<typeof PoolSlots> = {
  title: "Compositions/Pool slots",
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
const TIP = "9f1c2ab84d5e";
const COST = { name: "armada/job-a", commits: 3, tip: TIP, base: "main" };

/**
 * A Job's worktree outside the pool is a tile after the bays, in the same
 * grid: its Job as a link, its branch, and how long it has sat. **Its state is
 * what holds it, in git's words**: held while the Job has not finished, kept for
 * what it holds, free where nothing is uncommitted or unmerged, removed where
 * only the branch or record is left.
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
    await expect(tile(canvas, "run").getByRole("img", { name: "Held: its Job has not finished" })).toBeInTheDocument();
    await expect(tile(canvas, "kept").getByRole("img", { name: "Kept: uncommitted changes" })).toBeInTheDocument();
    await expect(tile(canvas, "free").getByRole("img", { name: "Free: nothing uncommitted or unmerged" })).toBeInTheDocument();
    await expect(tile(canvas, "gone").getByRole("img", { name: "Kept: unmerged commits" })).toBeInTheDocument();
    await expect(tile(canvas, "record").getByRole("img", { name: "Worktree removed" })).toBeInTheDocument();
    await expect(tile(canvas, "kept").getByLabelText("Last moved 4 days ago")).toHaveTextContent("4 days");
    await userEvent.click(tile(canvas, "kept").getByRole("button", { name: "Port the settings selectors" }));
    await expect(args.onOpenJob).toHaveBeenCalledWith("KEPT");
    // The Job link is a link and not a way into the panel.
    await expect(canvas.queryByRole("dialog")).toBeNull();
  },
};

/**
 * The panel lists every reason a worktree is held as a short row, a mark and a
 * word, with what it carries under it. **Every value says what it is**:
 * uncommitted changes name their files and how long ago the Job last moved,
 * unmerged commits say what they are not on and their tip, and a Job that has
 * not finished shows its status badge under the Job's name. A worktree that
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
      outside({ job_id: "RUN", status: "running", held: [{ why: "not_terminal", status: "running" }] }, { job: "5-fix-the-reader" }),
    ],
  },
  play: async ({ canvas, userEvent }) => {
    const all = await openTile(canvas, userEvent, "all");
    const holds = within(all.getByRole("region", { name: "What it holds" }));
    await expect(holds.getAllByRole("listitem").filter((one) => one.className.includes("armada-tile-holds__row"))).toHaveLength(6);
    await expect(holds.getByText("Uncommitted changes")).toBeInTheDocument();
    await expect(holds.getByLabelText("Uncommitted files")).toHaveTextContent("src/a.rs");
    await expect(holds.getByLabelText("Uncommitted files")).toHaveTextContent("notes/b.md");
    await expect(holds.getByText("Job last moved 4 days ago")).toBeInTheDocument();
    await expect(holds.getByText("Unmerged commits")).toBeInTheDocument();
    await expect(holds.getByText("3 commits not on main")).toBeInTheDocument();
    await expect(holds.getByText("tip 9f1c2ab84d5e")).toBeInTheDocument();
    await expect(holds.getByText("Worktree locked")).toBeInTheDocument();
    await expect(holds.getByText("claimed by a Pilot")).toBeInTheDocument();
    await expect(holds.getByText("Needed by unfinished Jobs")).toBeInTheDocument();
    await expect(holds.getByLabelText("Needed by")).toHaveTextContent("job-handle");
    await expect(holds.getByText("Base branch unknown")).toBeInTheDocument();
    await expect(holds.getByText("origin/main not found")).toBeInTheDocument();
    await expect(holds.getByText("git status failed", { selector: ".armada-tile-holds__word" })).toBeInTheDocument();
    await userEvent.keyboard("{Escape}");

    const nothing = await openTile(canvas, userEvent, "nothing");
    await expect(nothing.queryByRole("region", { name: "What it holds" })).toBeNull();
    await userEvent.keyboard("{Escape}");

    // The Job's status is said to be the Job's, in the Board's own badge.
    const running = await openTile(canvas, userEvent, "run");
    const row = within(running.getByRole("region", { name: "What it holds" }));
    await expect(row.getByText("Job 5-fix-the-reader")).toBeInTheDocument();
    await expect(row.getByLabelText("Job status: running")).toHaveTextContent("running");
    await expect(row.getByLabelText("Job status: running").querySelector(".armada-badge")).not.toBeNull();
    await expect(row.getByLabelText("Job status: running").closest(".armada-tooltip")).toHaveAccessibleDescription(
      "Job status: running. It has not finished, so nothing here can be cleared",
    );
  },
};

/**
 * **Clear is sent only from its confirm**, which lists the git effects it will
 * have: the worktree it removes, the uncommitted files it deletes with how long
 * ago the Job last moved, and the branch it keeps with its commits not on main.
 * Cancel sends nothing.
 */
export const ClearConfirm: Story = {
  name: "Clear confirm",
  args: {
    rows: [
      outside(
        { job_id: "DIRTY", held: [{ why: "uncommitted", files: ["src/a.rs", "notes/b.md"] }, UNMERGED] },
        { offered: CLEAR, cost: { files: ["src/a.rs", "notes/b.md"], branch: COST } },
      ),
    ],
    onClear: fn(),
  },
  play: async ({ args, canvas, userEvent }) => {
    const open = await openTile(canvas, userEvent, "dirty");
    await userEvent.click(open.getByRole("button", { name: "Clear" }));
    await expect(args.onClear).not.toHaveBeenCalled();

    const confirm = within(canvas.getByRole("group", { name: "Clear dirty" }));
    await expect(confirm.getByText("Removes the worktree at /Users/user/armada/.armada/worktrees/01JOB0001")).toBeInTheDocument();
    await expect(confirm.getByText("Deletes uncommitted files")).toBeInTheDocument();
    await expect(confirm.getByRole("list", { name: "Uncommitted files" })).toHaveTextContent("src/a.rs");
    await expect(confirm.getByRole("list", { name: "Uncommitted files" })).toHaveTextContent("notes/b.md");
    await expect(confirm.getByText("Job last moved 4 days ago")).toBeInTheDocument();
    await expect(confirm.getByText("Keeps branch armada/job-a: 3 commits not on main")).toBeInTheDocument();
    await expect(confirm.getByRole("list", { name: "Branch tip" })).toHaveTextContent(TIP);
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

/**
 * **A bay is released, not removed.** Its worktree stays at its slot path on a
 * detached HEAD, and the pool refuses the release while anything is
 * uncommitted, so the confirm lists those files as what holds it up and never
 * as deleted.
 */
export const ClearConfirmOnABay: Story = {
  name: "Clear confirm on a bay",
  args: {
    rows: [
      {
        slot: slot(2, { held: { state: "job", job_id: "01JOB0001", job_title: "Port the settings selectors" }, branch: "armada/job-a" }),
        held: held({ branch: "armada/job-a", held: [{ why: "uncommitted", files: ["src/a.rs"] }, UNMERGED] }),
        sat: "4 days",
        offered: CLEAR,
        cost: { files: ["src/a.rs"], branch: COST },
      },
    ],
    onClear: fn(),
  },
  play: async ({ canvas, userEvent }) => {
    const open = await openTile(canvas, userEvent, "slot-2");
    await userEvent.click(open.getByRole("button", { name: "Clear" }));
    const confirm = within(canvas.getByRole("group", { name: "Clear slot-2" }));
    await expect(confirm.getByText("Releases slot-2 to the pool")).toBeInTheDocument();
    await expect(confirm.getByText("Keeps the worktree at .armada/slots/slot-2, detached from armada/job-a")).toBeInTheDocument();
    await expect(confirm.getByText("Refused while these are uncommitted")).toBeInTheDocument();
    await expect(confirm.queryByText("Deletes uncommitted files")).toBeNull();
    await expect(confirm.getByRole("list", { name: "Uncommitted files" })).toHaveTextContent("src/a.rs");
    await expect(confirm.queryByText(/^Removes the worktree/)).toBeNull();
  },
};

/** A worktree holding nothing uncommitted still confirms, and says what becomes of its branch. */
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
    await expect(confirm.queryByRole("list", { name: "Uncommitted files" })).toBeNull();
    await expect(confirm.queryByText("Deletes uncommitted files")).toBeNull();
    await expect(confirm.getByText("Deletes branch armada/01JOB0001: merged into main")).toBeInTheDocument();
  },
};

/** Delete branch names the branch and its tip, what stays reachable only from it, and sends that tip. */
export const DeleteBranchConfirm: Story = {
  name: "Delete branch confirm",
  args: {
    rows: [outside({ job_id: "GONE", on_disk: false, held: [UNMERGED] }, { offered: BRANCH, cost: { files: [], branch: COST } })],
    onDeleteBranch: fn(),
  },
  play: async ({ args, canvas, userEvent }) => {
    const open = await openTile(canvas, userEvent, "gone");
    await expect(open.queryByRole("button", { name: "Clear" })).toBeNull();
    await userEvent.click(open.getByRole("button", { name: "Delete branch" }));
    const confirm = within(canvas.getByRole("group", { name: "Delete branch gone" }));
    await expect(confirm.getByText(`Deletes branch armada/job-a at ${TIP}`)).toBeInTheDocument();
    await expect(confirm.getByText(`3 commits not on main stay reachable only from ${TIP}`)).toBeInTheDocument();
    await expect(args.onDeleteBranch).not.toHaveBeenCalled();
    await userEvent.click(confirm.getByRole("button", { name: "Delete branch" }));
    await expect(args.onDeleteBranch).toHaveBeenCalledWith("GONE", TIP);
  },
};

/** Forget Job names the Job whose record goes, and is sent only from its confirm. */
export const ForgetConfirm: Story = {
  name: "Forget confirm",
  args: {
    rows: [outside({ job_id: "OLD", on_disk: false }, { offered: { clear: false, deleteBranch: false, forget: true }, job: "7-old-job" })],
    onForget: fn(),
  },
  play: async ({ args, canvas, userEvent }) => {
    const open = await openTile(canvas, userEvent, "old");
    await userEvent.click(open.getByRole("button", { name: "Forget Job" }));
    const confirm = within(canvas.getByRole("group", { name: "Forget Job old" }));
    await expect(confirm.getByText("Deletes the record of Job 7-old-job")).toBeInTheDocument();
    await expect(confirm.getByText("Leaves the worktree and branch alone")).toBeInTheDocument();
    await expect(args.onForget).not.toHaveBeenCalled();
    await userEvent.click(confirm.getByRole("button", { name: "Forget Job" }));
    await expect(args.onForget).toHaveBeenCalledWith("OLD");
  },
};

/**
 * **Each act's tooltip says what it acts on and what happens to it**, in git's
 * words, checked against `reclaim_worktree`, `delete_branch`, `forget_job` and
 * the pool: a bay is released and its directory stays; a worktree outside the
 * pool is removed; a branch is deleted only where the base has all its commits;
 * Delete branch names the tip its commits stay reachable from.
 */
export const ActTooltips: Story = {
  name: "Act tooltips",
  args: {
    rows: [
      {
        slot: slot(2, { held: { state: "job", job_id: "01JOB0001", job_title: "Port the settings selectors" }, branch: "armada/job-a" }),
        held: held({ branch: "armada/job-a", held: [UNMERGED] }),
        offered: CLEAR,
        cost: { files: [], branch: COST },
        job: "7-port-the-settings-selectors",
      },
      outside({ job_id: "OUT", branch: "armada/out", path: "/r/.armada/worktrees/OUT" }, { offered: CLEAR }),
      outside({ job_id: "BRANCH", on_disk: false, branch: "armada/b", held: [UNMERGED] }, { offered: BRANCH, cost: { files: [], branch: { ...COST, name: "armada/b" } } }),
      outside({ job_id: "OLD", on_disk: false }, { offered: { clear: false, deleteBranch: false, forget: true }, job: "7-old-job" }),
      { slot: slot(3, { held: { state: "free" }, closed: true }) },
      { slot: slot(4, { held: { state: "free" } }) },
      { slot: slot(5, STRANDED) },
      { slot: slot(6, { ...STRANDED, rescue: { ...FOUND, state: "reading" } }) },
    ],
    onAct: fn(),
    onRescue: fn(),
    onClear: fn(),
    onDeleteBranch: fn(),
    onForget: fn(),
  },
  play: async ({ canvas, userEvent }) => {
    const tip = async (tileName: string, act: string, said: string) => {
      const open = await openTile(canvas, userEvent, tileName);
      await expect(open.getAllByRole("button", { name: act })[0]).toHaveAccessibleDescription(said);
      await userEvent.keyboard("{Escape}");
    };
    await tip(
      "slot-2",
      "Clear",
      "Releases slot-2 to the pool: detaches its worktree at .armada/slots/slot-2 from armada/job-a and keeps the directory. Refused while it has uncommitted changes. Deletes branch armada/job-a only if main has all its commits, otherwise keeps it.",
    );
    await tip(
      "slot-2",
      "Close slot",
      "Closes slot-2: no new lease will take it. Its holder keeps it until its lease ends.",
    );
    await tip(
      "out",
      "Clear",
      "Removes the worktree at /r/.armada/worktrees/OUT. Deletes branch armada/out only if main has all its commits, otherwise keeps it.",
    );
    await tip(
      "branch",
      "Delete branch",
      `Deletes branch armada/b at ${TIP}. Its commits not on main stay reachable only from that commit.`,
    );
    await tip("old", "Forget Job", "Deletes the record of Job 7-old-job. The worktree and branch are not touched.");
    await tip("slot-3", "Reopen slot", "Reopens slot-3: a new lease can take it again.");
    await tip(
      "slot-4",
      "Remove slot",
      "Removes slot-4 from the pool with git worktree remove on .armada/slots/slot-4. Refused while it holds uncommitted changes.",
    );
    await tip(
      "slot-5",
      "Rescue",
      "Starts a Scout that reads the uncommitted changes and the commits not on main in slot-5. It changes nothing.",
    );
    await tip("slot-6", "Stop", "Stops the Scout reading slot-6.");
  },
};

/**
 * Acts are offered only where they apply. **A Job that has not finished offers
 * none of the three**, a worktree whose checkout is gone offers no Clear, and a
 * pool drawn with none of the reclaim wiring offers none either. The slot's
 * Close reads as closing the slot and the sheet's as closing the panel.
 */
export const OfferedWhereTheyApply: Story = {
  name: "Offered where they apply",
  args: {
    rows: [
      outside({ job_id: "RUN", held: [{ why: "not_terminal", status: "running" }] }),
      outside({ job_id: "KEPT", held: [UNMERGED] }, { offered: { clear: true, deleteBranch: false, forget: false } }),
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
    await expect(await offered("kept")).toEqual(["Clear"]);
    await expect(await offered("gone")).toEqual(["Forget Job"]);
  },
};

/**
 * Each act reads as a button in the hue of what it does: destructive acts in
 * the error red, a rescue in the warning amber, Close and Reopen in the
 * default text. A label sits beside the icon, and a bare icon is never the act.
 */
export const ActsByTone: Story = {
  name: "Acts by tone",
  args: {
    rows: [
      outside({ job_id: "OUT" }, { offered: CLEAR }),
      { slot: slot(5, STRANDED) },
    ],
    onAct: fn(),
    onRescue: fn(),
    onClear: fn(),
  },
  play: async ({ canvas, userEvent }) => {
    const out = await openTile(canvas, userEvent, "out");
    const clear = out.getByRole("button", { name: "Clear" });
    await expect(clear).toHaveTextContent("Clear");
    await expect(clear.querySelector("svg")?.getAttribute("width")).toBe("16");
    await expect(clear).toHaveAttribute("data-tone", "destructive");
    await userEvent.keyboard("{Escape}");
    const bay = await openTile(canvas, userEvent, "slot-5");
    await expect(bay.getByRole("button", { name: "Rescue" })).toHaveAttribute("data-tone", "rescue");
    await expect(bay.getByRole("button", { name: "Close slot" })).toHaveAttribute("data-tone", "neutral");
    await expect(getComputedStyle(bay.getByRole("button", { name: "Close slot" })).borderTopStyle).toBe("solid");
    // The slot act and the panel's own close are told apart by name.
    await expect(bay.getByRole("button", { name: "Close panel Esc" })).toBeInTheDocument();
  },
};

/** What Clear did and what Fleet refused are said in the panel, a fact to a line, and never on the tile. */
export const ReceiptAndRefusal: Story = {
  name: "Receipt and refusal",
  args: {
    rows: [
      outside({ job_id: "DONE" }, { offered: CLEAR, receipt: ["Worktree removed", "Branch kept: 3 commits not on main"] }),
      outside({ job_id: "NO" }, { offered: CLEAR, refused: "Worktree not removed: the Job has not finished" }),
    ],
  },
  play: async ({ canvas, userEvent }) => {
    await expect(canvas.queryByRole("status")).toBeNull();
    await expect(canvas.queryByRole("alert")).toBeNull();
    const done = await openTile(canvas, userEvent, "done");
    await expect(done.getByRole("status")).toHaveTextContent("Worktree removed");
    await expect(done.getByRole("status")).toHaveTextContent("Branch kept: 3 commits not on main");
    await userEvent.keyboard("{Escape}");
    const no = await openTile(canvas, userEvent, "no");
    await expect(no.getByRole("alert")).toHaveTextContent("Worktree not removed: the Job has not finished");
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
    await expect(open.getByRole("button", { name: "Close slot" })).toBeInTheDocument();
    await expect(open.getByRole("region", { name: "What it holds" })).toHaveTextContent("src/a.rs");
    await expect(open.getByRole("button", { name: "Port the settings selectors" })).toBeInTheDocument();
  },
};
