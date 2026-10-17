// A stranded slot's Finding gets a third act: Pick up stashes the work and frees the slot.

import { dialog, inside, role, viaPalette, walk } from "../walk";

const BAY = (n: number) => role("listitem", `slot-${n}`, { exact: true });
const TILE = (n: number) => inside(BAY(n), role("button", `slot-${n}`, { exact: true }));
const PANEL = dialog("slot-4");
const IN_PANEL = (name: string) => inside(PANEL, role("button", name, { exact: true }));

export const pickingUpASlot = walk("cleanup/slots", [
  ...viaPalette("Worktree Slots", "Cleanup, from the rail"),
  { press: TILE(4), say: "The stranded slot's panel opens" },
  { press: IN_PANEL("Rescue"), say: "A Scout starts reading it" },
  { look: inside(PANEL, role("heading", "Unfinished", { exact: true })), say: "Unfinished, with what is left" },
  { hover: IN_PANEL("Pick up"), say: "Pick up is beside Stash and Scrap" },
  { press: IN_PANEL("Pick up"), say: "Picked up at once" },
  { look: inside(PANEL, role("status")), say: "The slot is free, and the commit is named in the panel" },
  { press: IN_PANEL("Close panel Esc"), say: "Close the panel" },
]);
