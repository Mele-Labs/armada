// The worktree pool reshaped from Cleanup's grid: a slot closed and reopened
// from its panel, a held one closed under its Job, one added and removed, and a
// remove Fleet refused, said in the panel of the slot it was about.

import { dialog, inside, role, walk } from "../walk";

const BAY = (n: number) => role("listitem", `slot-${n}`, { exact: true });
const TILE = (n: number) => inside(BAY(n), role("button", `slot-${n}`, { exact: true }));
const PANEL = (n: number) => dialog(`slot-${n}`);
const ACT = (n: number, said: string) => inside(PANEL(n), role("button", said, { exact: true }));
const SHUT = (n: number) => inside(PANEL(n), role("button", "Close panel Esc", { exact: true }));

export const reshapingTheSlotPool = walk("cleanup/slots", [
  { press: role("button", "Cleanup", { exact: true }), say: "Cleanup, from the rail" },
  { look: role("list", "Worktree slots"), say: "The pool, a bay per slot, and a tile to add one" },
  { press: TILE(3), say: "A press on a bay opens its panel" },
  { hover: ACT(3, "Close slot"), say: "Close: no lease takes this slot until it is reopened" },
  { press: ACT(3, "Close slot"), say: "Closed" },
  { press: SHUT(3), say: "Back to the grid" },
  { look: BAY(3), say: "A closed bay is shuttered and locked" },
  { press: TILE(3), say: "Its panel again" },
  { press: ACT(3, "Reopen slot"), say: "Reopened, it is free again" },
  { press: SHUT(3), say: "Back to the grid" },
  { press: TILE(1), say: "A slot a Job holds" },
  { press: ACT(1, "Close slot"), say: "Close it" },
  { press: SHUT(1), say: "Back to the grid" },
  { look: BAY(1), say: "Its Job keeps it until the lease ends; the shutter marks it closed" },
  { hover: inside(BAY(1), role("img", "Closed", { exact: true })), say: "The closed mark, named" },
  { press: TILE(1), say: "Its panel" },
  { press: ACT(1, "Reopen slot"), say: "Reopened" },
  { press: SHUT(1), say: "Back to the grid" },
  { hover: role("button", "Add a slot", { exact: true }), say: "Add a slot" },
  { press: role("button", "Add a slot", { exact: true }), say: "Added" },
  { look: BAY(9), say: "The new slot, not made until a lease makes it" },
  { press: TILE(9), say: "Its panel" },
  { press: ACT(9, "Remove slot"), say: "Remove it again" },
  { look: role("list", "Worktree slots"), say: "Gone, and its panel with it" },
  { press: TILE(5), say: "A free slot something was written to since the read" },
  { press: ACT(5, "Remove slot"), say: "Remove it" },
  { look: inside(PANEL(5), role("alert")), say: "Refused, in its panel: what is in it" },
]);
