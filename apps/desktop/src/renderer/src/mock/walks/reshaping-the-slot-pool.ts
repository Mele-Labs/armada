// The worktree pool reshaped from Cleanup's bay grid: a slot closed and
// reopened, a held one closed under its Job, one added and removed, and a
// remove Fleet refused, said on the slot it was about.

import { inside, role, walk } from "../walk";

const BAY = (n: number) => role("listitem", `slot-${n}`, { exact: true });
const ACT = (n: number, said: string) => inside(BAY(n), role("button", said, { exact: true }));

export const reshapingTheSlotPool = walk("cleanup/slots", [
  { press: role("button", "Cleanup", { exact: true }), say: "Cleanup, from the rail" },
  { look: role("list", "Worktree slots"), say: "The pool, a bay per slot, and a tile to add one" },
  { hover: ACT(3, "Close"), say: "Close: no lease takes this slot until it is reopened" },
  { press: ACT(3, "Close"), say: "Closed" },
  { look: BAY(3), say: "A closed bay is shuttered and locked" },
  { press: ACT(3, "Reopen"), say: "Reopened, it is free again" },
  { press: ACT(1, "Close"), say: "Close a slot a Job holds" },
  { look: BAY(1), say: "Its Job keeps it until the lease ends; the shutter marks it closed" },
  { hover: inside(BAY(1), role("img", "Closed", { exact: true })), say: "The closed mark, named" },
  { press: ACT(1, "Reopen"), say: "Reopened" },
  { hover: role("button", "Add a slot", { exact: true }), say: "Add a slot" },
  { press: role("button", "Add a slot", { exact: true }), say: "Added" },
  { look: BAY(7), say: "The new slot, not made until a lease makes it" },
  { press: ACT(7, "Remove"), say: "Remove it again" },
  { look: role("list", "Worktree slots"), say: "Gone" },
  { press: ACT(5, "Remove"), say: "Remove a free slot something was written to since the read" },
  { look: inside(BAY(5), role("alert")), say: "Refused, on the slot: what is in it" },
]);
