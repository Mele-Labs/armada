// A stranded slot on Cleanup rescued: a Scout reads it, its Finding opens in the
// trailing sheet from a button on the bay, and the owner stashes it. A second
// stranded bay shows the Scrap's confirm, which names the files and unpushed
// commits that would go.

import { dialog, inside, role, walk } from "../walk";

const BAY = (n: number) => role("listitem", `slot-${n}`, { exact: true });
const ACT = (n: number, said: string) => inside(BAY(n), role("button", said, { exact: true }));
const FINDING = dialog("Finding");
const IN_FINDING = (kind: string, name: string) => inside(FINDING, role(kind, name, { exact: true }));

export const rescuingAStrandedSlot = walk("cleanup/slots", [
  { press: role("button", "Cleanup", { exact: true }), say: "Cleanup, from the rail" },
  { look: BAY(4), say: "A stranded slot: hatched, with its holder gone" },
  { hover: ACT(4, "Rescue"), say: "Rescue" },
  { press: ACT(4, "Rescue"), say: "A Scout starts reading it" },
  { look: inside(BAY(4), role("img", "Reading", { exact: true })), say: "Reading: the mark pulses on the bay" },
  { hover: ACT(4, "Stop"), say: "Stop ends the read" },
  { press: ACT(4, "Finding"), say: "The Finding opens in the side panel, and the files appear as it reads" },
  { look: inside(FINDING, role("list", "Searched", { exact: true })), say: "The Finding: what it read and searched" },
  { hover: IN_FINDING("img", "Uncommitted changes on top"), say: "Uncommitted changes were on top of the commit it read" },
  { hover: IN_FINDING("button", "Scrap"), say: "Scrap and Stash are in the panel" },
  { hover: IN_FINDING("button", "Stash"), say: "Stash commits the uncommitted work to its branch" },
  { press: IN_FINDING("button", "Stash"), say: "Stashed, at once, and the panel closes" },
  { look: inside(BAY(4), role("status")), say: "The slot is free, and the commit is named" },
  { press: ACT(7, "Rescue"), say: "The second stranded slot, rescued" },
  { press: ACT(7, "Finding"), say: "Its Finding, in the panel" },
  { look: inside(FINDING, role("list", "Searched", { exact: true })), say: "What it read" },
  { press: IN_FINDING("button", "Scrap"), say: "Scrap" },
  { look: role("group", "Scrap slot-7", { exact: true }), say: "It names the uncommitted files and the unpushed commits, and nothing is sent until it is confirmed" },
]);
