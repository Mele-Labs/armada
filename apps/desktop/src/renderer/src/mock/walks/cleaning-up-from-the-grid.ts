// Cleanup as one grid of tiles. A press on a tile opens the panel that manages
// that worktree: what it holds, and the acts that fit it. A slot an agent session
// holds, in a terminal or hosted, is released, with its files committed first or
// with none, and refused while a hosted one is working; a stranded bay is rescued
// and a closed one reopened.

import { dialog, inside, role, viaPalette, walk } from "../walk";

const BAY = (n: number) => role("listitem", `slot-${n}`, { exact: true });
const TILE = (n: number) => inside(BAY(n), role("button", `slot-${n}`, { exact: true }));
const PANEL = (n: number) => dialog(`slot-${n}`);
const IN_PANEL = (n: number, kind: string, name: string) => inside(PANEL(n), role(kind, name, { exact: true }));
const SHUT = (n: number) => IN_PANEL(n, "button", "Close panel Esc");

export const cleaningUpFromTheGrid = walk("cleanup/grid", [
  ...viaPalette("Worktree Slots", "Cleanup, from the rail"),
  { look: role("list", "Worktree slots"), say: "One grid: the pool's bays and a tile to add one" },
  { press: TILE(2), say: "A bay held by an agent job, with files it did not commit" },
  { press: IN_PANEL(2, "button", "Release"), say: "Release asks first" },
  { look: role("group", "Release slot-2", { exact: true }), say: "It names the holder and its branch, commits the files to the branch as a WIP commit, releases the slot and keeps the branch" },
  { press: inside(role("group", "Release slot-2", { exact: true }), role("button", "Release", { exact: true })), say: "Sent" },
  { look: inside(PANEL(2), role("status")), say: "Committed to the branch, slot released" },
  { press: SHUT(2), say: "Back to the grid" },
  { press: TILE(6), say: "A bay held by a job with nothing uncommitted" },
  { press: IN_PANEL(6, "button", "Release"), say: "Release asks first" },
  { look: role("group", "Release slot-6", { exact: true }), say: "It releases the slot and keeps the branch, with no files to commit" },
  { press: inside(role("group", "Release slot-6", { exact: true }), role("button", "Release", { exact: true })), say: "Sent" },
  { look: inside(PANEL(6), role("status")), say: "Slot released" },
  { press: SHUT(6), say: "Back to the grid" },
  { press: TILE(9), say: "A bay a hosted Job holds, between turns" },
  { press: IN_PANEL(9, "button", "Release"), say: "Release asks first, as it does for a terminal's" },
  { press: inside(role("group", "Release slot-9", { exact: true }), role("button", "Release", { exact: true })), say: "Sent" },
  { look: inside(PANEL(9), role("status")), say: "Slot released" },
  { press: SHUT(9), say: "Back to the grid" },
  { press: TILE(10), say: "A bay a hosted Job holds, mid-turn" },
  { press: IN_PANEL(10, "button", "Release"), say: "Release asks first" },
  { press: inside(role("group", "Release slot-10", { exact: true }), role("button", "Release", { exact: true })), say: "Sent" },
  { look: inside(PANEL(10), role("alert")), say: "Refused: the Job is working" },
  { look: IN_PANEL(10, "button", "Open Job"), say: "Open Job goes to it" },
  { press: SHUT(10), say: "Back to the grid" },
  { press: TILE(4), say: "A stranded bay" },
  { press: IN_PANEL(4, "button", "Rescue"), say: "Rescue sends a Scout to read it" },
  { look: IN_PANEL(4, "heading", "Unfinished"), say: "Its Finding is in the same panel" },
  { press: SHUT(4), say: "Back to the grid" },
  { press: TILE(3), say: "A closed bay" },
  { press: IN_PANEL(3, "button", "Reopen slot"), say: "Reopen" },
  { look: BAY(3), say: "Free again" },
]);
