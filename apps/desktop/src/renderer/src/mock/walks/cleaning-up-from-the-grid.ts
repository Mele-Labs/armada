// Cleanup as one grid of tiles. A press on a tile opens the panel that manages
// that worktree: what it holds, and the acts that fit it. A finished Job's bay
// is cleared after a confirm that names the files it destroys; a running Job's
// holds the tree and offers nothing; two worktrees outside the pool sit after
// the bays, one with a branch to delete; a stranded bay is rescued and a closed
// one reopened. The old list below the bays, and its bulk press, are gone.

import { dialog, inside, role, walk } from "../walk";

const BAY = (n: number) => role("listitem", `slot-${n}`, { exact: true });
const TILE = (n: number) => inside(BAY(n), role("button", `slot-${n}`, { exact: true }));
const PANEL = (n: number) => dialog(`slot-${n}`);
const IN_PANEL = (n: number, kind: string, name: string) => inside(PANEL(n), role(kind, name, { exact: true }));
const SHUT = (n: number) => IN_PANEL(n, "button", "Close Esc");

/** A worktree outside the pool, named by its Job's handle. */
const OUTSIDE = (handle: RegExp) => role("button", handle);
const OUTSIDE_PANEL = (handle: RegExp) => dialog(handle);
const SHUT_OUTSIDE = (handle: RegExp) => inside(OUTSIDE_PANEL(handle), role("button", "Close Esc", { exact: true }));

const REJECTED = /-rejected$/;
const FAILED = /-completedFailed$/;

export const cleaningUpFromTheGrid = walk("cleanup/grid", [
  { press: role("button", "Cleanup", { exact: true }), say: "Cleanup, from the rail" },
  { look: role("list", "Worktree slots"), say: "One grid: the pool's bays, a tile to add one, and the Jobs' worktrees outside it" },
  { press: TILE(1), say: "A finished Job's bay opens its panel" },
  { look: inside(PANEL(1), role("region", "What it holds", { exact: true })), say: "What it holds: files nobody committed, and a branch nothing merged" },
  { hover: IN_PANEL(1, "button", "Clear"), say: "Clear gives the checkout back" },
  { press: IN_PANEL(1, "button", "Clear"), say: "Clear asks first" },
  { look: role("group", "Clear slot-1", { exact: true }), say: "It names the files it destroys and how long they have sat, and the branch it keeps" },
  { press: inside(role("group", "Clear slot-1", { exact: true }), role("button", "Clear", { exact: true })), say: "Cleared" },
  { look: inside(PANEL(1), role("status")), say: "What it did: the checkout gone, the branch kept" },
  { press: SHUT(1), say: "Back to the grid" },
  { press: TILE(5), say: "A running Job's bay" },
  { look: inside(PANEL(5), role("region", "What it holds", { exact: true })), say: "It holds the tree while it runs, and the panel offers nothing to do" },
  { press: SHUT(5), say: "Back to the grid" },
  { press: OUTSIDE(REJECTED), say: "A worktree outside the pool, in the same grid, opens the same panel" },
  { look: inside(OUTSIDE_PANEL(REJECTED), role("region", "What it holds", { exact: true })), say: "Uncommitted files, and an unmerged branch" },
  { press: SHUT_OUTSIDE(REJECTED), say: "Back to the grid" },
  { press: OUTSIDE(FAILED), say: "A worktree whose checkout is gone and whose branch is left" },
  { press: inside(OUTSIDE_PANEL(FAILED), role("button", "Delete branch", { exact: true })), say: "Delete branch asks first" },
  { look: role("group", /^Delete branch /), say: "It names the branch, its commits and the tip they stay reachable from" },
  { press: inside(role("group", /^Delete branch /), role("button", "Delete branch", { exact: true })), say: "Deleted" },
  { look: inside(OUTSIDE_PANEL(FAILED), role("status")), say: "The branch is gone, at its tip" },
  { hover: inside(OUTSIDE_PANEL(FAILED), role("button", "Forget Job", { exact: true })), say: "With nothing left standing, Forget Job is offered" },
  { press: SHUT_OUTSIDE(FAILED), say: "Back to the grid" },
  { press: TILE(4), say: "A stranded bay" },
  { press: IN_PANEL(4, "button", "Rescue"), say: "Rescue sends a Scout to read it" },
  { look: IN_PANEL(4, "heading", "Unfinished"), say: "Its Finding is in the same panel" },
  { press: SHUT(4), say: "Back to the grid" },
  { press: TILE(3), say: "A closed bay" },
  { press: IN_PANEL(3, "button", "Reopen"), say: "Reopen" },
  { look: BAY(3), say: "Free again" },
]);
