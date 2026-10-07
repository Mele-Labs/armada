// A stranded slot on Cleanup rescued from its panel: a Scout reads it, its
// Finding is in the same panel, and the owner stashes it. A second stranded bay
// shows the Scrap's confirm, which names the files and unpushed commits that
// would go.

import { dialog, inside, role, text, walk } from "../walk";

const BAY = (n: number) => role("listitem", `slot-${n}`, { exact: true });
const TILE = (n: number) => inside(BAY(n), role("button", `slot-${n}`, { exact: true }));
const PANEL = (n: number) => dialog(`slot-${n}`);
const IN_PANEL = (n: number, kind: string, name: string) => inside(PANEL(n), role(kind, name, { exact: true }));
const SHUT = (n: number) => IN_PANEL(n, "button", "Close panel Esc");

export const rescuingAStrandedSlot = walk("cleanup/slots", [
  { press: role("button", "Worktree Slots", { exact: true }), say: "Cleanup, from the rail" },
  { look: BAY(4), say: "A stranded slot: hatched, with its holder gone" },
  { press: TILE(4), say: "Its panel opens" },
  { hover: IN_PANEL(4, "button", "Rescue"), say: "Rescue" },
  { press: IN_PANEL(4, "button", "Rescue"), say: "A Scout starts reading it" },
  { look: IN_PANEL(4, "img", "Reading"), say: "Reading: the files appear in the panel as it goes" },
  { hover: IN_PANEL(4, "button", "Stop"), say: "Stop ends the read" },
  { look: IN_PANEL(4, "heading", "Unfinished"), say: "The verdict is one word: there is a part left to do" },
  { look: inside(PANEL(4), role("list", "Left to do", { exact: true })), say: "What is left, a line each" },
  { look: inside(PANEL(4), role("list", "Searched", { exact: true })), say: "What it read and searched" },
  { hover: inside(PANEL(4), text("Only here")), say: "Only here: on no remote branch and not on main. Those commits come first" },
  { hover: IN_PANEL(4, "img", "Uncommitted changes on top"), say: "Uncommitted changes were on top of the commit it read" },
  { hover: IN_PANEL(4, "button", "Scrap"), say: "Scrap and Stash are in the panel" },
  { hover: IN_PANEL(4, "button", "Stash"), say: "Stash commits the uncommitted work to its branch" },
  { press: IN_PANEL(4, "button", "Stash"), say: "Stashed, at once" },
  { look: inside(PANEL(4), role("status")), say: "The slot is free, and the commit is named in the panel" },
  { press: SHUT(4), say: "Back to the grid" },
  { press: TILE(7), say: "The second stranded slot" },
  { press: IN_PANEL(7, "button", "Rescue"), say: "A Scout reads it" },
  { look: IN_PANEL(7, "heading", "Scraps"), say: "Scraps: leftovers that need no more work, in one line" },
  { press: IN_PANEL(7, "button", "Scrap"), say: "Scrap" },
  { look: role("group", "Scrap slot-7", { exact: true }), say: "It names the uncommitted files and the unpushed commits, and nothing is sent until it is confirmed" },
]);
