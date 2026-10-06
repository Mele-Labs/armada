// Cleanup as one grid of tiles. A press on a tile opens the panel that manages
// that worktree: what it holds, and the acts that fit it. A finished Job's bay
// is released after a confirm that says its uncommitted files are committed to
// the Job's branch as a WIP commit; a worktree outside the pool is committed to
// the same way and then removed; the Job's own Clear says the same; a slot an
// agent session holds is released, with its files committed first or with none; a running Job's
// holds the tree and offers nothing; two worktrees outside the pool sit after
// the bays, one with a branch to delete; a stranded bay is rescued and a closed
// one reopened. The old list below the bays, and its bulk press, are gone.

import { dialog, inside, role, walk } from "../walk";

const BAY = (n: number) => role("listitem", `slot-${n}`, { exact: true });
const TILE = (n: number) => inside(BAY(n), role("button", `slot-${n}`, { exact: true }));
const PANEL = (n: number) => dialog(`slot-${n}`);
const IN_PANEL = (n: number, kind: string, name: string) => inside(PANEL(n), role(kind, name, { exact: true }));
const SHUT = (n: number) => IN_PANEL(n, "button", "Close panel Esc");

/** A worktree outside the pool, named by its Job's handle. */
const OUTSIDE = (handle: RegExp) => role("button", handle);
const OUTSIDE_PANEL = (handle: RegExp) => dialog(handle);
const SHUT_OUTSIDE = (handle: RegExp) => inside(OUTSIDE_PANEL(handle), role("button", "Close panel Esc", { exact: true }));

const REJECTED = /-rejected$/;
const FAILED = /-completedFailed$/;

export const cleaningUpFromTheGrid = walk("cleanup/grid", [
  { press: role("button", "Cleanup", { exact: true }), say: "Cleanup, from the rail" },
  { look: role("list", "Worktree slots"), say: "One grid: the pool's bays, a tile to add one, and the Jobs' worktrees outside it" },
  { press: TILE(1), say: "A finished Job's bay opens its panel" },
  { look: inside(PANEL(1), role("region", "What it holds", { exact: true })), say: "What it holds: uncommitted changes, and commits not on main" },
  { press: inside(PANEL(1), role("button", /Debounce the Job Board/)), say: "The bay's Job opens over Cleanup" },
  { press: role("button", "Reclaim worktree"), say: "The Job's own Clear, on the same worktree" },
  { look: dialog("Give this job's worktree back?"), say: "It says the same: the uncommitted files committed to the branch, the slot released, the branch kept" },
  { press: inside(dialog("Give this job's worktree back?"), role("button", "Cancel", { exact: true })), say: "Cancel sends nothing" },
  { press: role("button", "Cleanup", { exact: true }), say: "Back to Cleanup" },
  { press: TILE(1), say: "The bay again" },
  { hover: IN_PANEL(1, "button", "Clear"), say: "Clear says which worktree it acts on and what happens to the branch" },
  { press: IN_PANEL(1, "button", "Clear"), say: "Clear asks first" },
  { look: role("group", "Clear slot-1", { exact: true }), say: "It says what will happen: the uncommitted files committed to the branch as a WIP commit, the slot released, the branch kept" },
  { press: inside(role("group", "Clear slot-1", { exact: true }), role("button", "Clear", { exact: true })), say: "Sent" },
  { look: inside(PANEL(1), role("status")), say: "Committed to the branch, slot released" },
  { press: SHUT(1), say: "Back to the grid" },
  { press: TILE(5), say: "A running Job's bay" },
  { look: inside(PANEL(5), role("region", "What it holds", { exact: true })), say: "Its row is the Job's status, as the Board draws it, and the panel offers nothing to do" },
  { hover: inside(PANEL(5), role("img", "Job status: running", { exact: true })), say: "Job status: running" },
  { press: SHUT(5), say: "Back to the grid" },
  { press: OUTSIDE(REJECTED), say: "A worktree outside the pool, in the same grid, opens the same panel" },
  { press: inside(OUTSIDE_PANEL(REJECTED), role("button", "Clear", { exact: true })), say: "Clear on a worktree outside the pool" },
  { look: role("group", /^Clear /), say: "It commits the uncommitted files it lists to the branch, removes the worktree at its path, and keeps the branch" },
  { press: inside(role("group", /^Clear /), role("button", "Clear", { exact: true })), say: "Sent" },
  { look: inside(OUTSIDE_PANEL(REJECTED), role("status")), say: "Committed to the branch, worktree removed" },
  { press: SHUT_OUTSIDE(REJECTED), say: "Back to the grid" },
  { press: OUTSIDE(FAILED), say: "A worktree already removed, with its branch left" },
  { press: inside(OUTSIDE_PANEL(FAILED), role("button", "Delete branch", { exact: true })), say: "Delete branch asks first" },
  { look: role("group", /^Delete branch /), say: "It names the branch, the commits not on main and the tip they stay reachable from" },
  { press: inside(role("group", /^Delete branch /), role("button", "Delete branch", { exact: true })), say: "Deleted" },
  { look: inside(OUTSIDE_PANEL(FAILED), role("status")), say: "Branch deleted at its tip" },
  { hover: inside(OUTSIDE_PANEL(FAILED), role("button", "Forget Job", { exact: true })), say: "With nothing left standing, Forget Job is offered" },
  { press: SHUT_OUTSIDE(FAILED), say: "Back to the grid" },
  { press: TILE(2), say: "A bay held by an agent session, with files it did not commit" },
  { press: IN_PANEL(2, "button", "Release"), say: "Release asks first" },
  { look: role("group", "Release slot-2", { exact: true }), say: "It names the holder and its branch, commits the files to the branch as a WIP commit, releases the slot and keeps the branch" },
  { press: inside(role("group", "Release slot-2", { exact: true }), role("button", "Release", { exact: true })), say: "Sent" },
  { look: inside(PANEL(2), role("status")), say: "Committed to the branch, slot released" },
  { press: SHUT(2), say: "Back to the grid" },
  { press: TILE(6), say: "A bay held by a session with nothing uncommitted" },
  { press: IN_PANEL(6, "button", "Release"), say: "Release asks first" },
  { look: role("group", "Release slot-6", { exact: true }), say: "It releases the slot and keeps the branch, with no files to commit" },
  { press: inside(role("group", "Release slot-6", { exact: true }), role("button", "Release", { exact: true })), say: "Sent" },
  { look: inside(PANEL(6), role("status")), say: "Slot released" },
  { press: SHUT(6), say: "Back to the grid" },
  { press: TILE(4), say: "A stranded bay" },
  { press: IN_PANEL(4, "button", "Rescue"), say: "Rescue sends a Scout to read it" },
  { look: IN_PANEL(4, "heading", "Unfinished"), say: "Its Finding is in the same panel" },
  { press: SHUT(4), say: "Back to the grid" },
  { press: TILE(3), say: "A closed bay" },
  { press: IN_PANEL(3, "button", "Reopen slot"), say: "Reopen" },
  { look: BAY(3), say: "Free again" },
]);
