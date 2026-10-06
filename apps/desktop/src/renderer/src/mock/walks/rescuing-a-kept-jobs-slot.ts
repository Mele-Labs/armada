// A Job that ended and kept its slot, beside the stranded ones: its own bay,
// the reason it was kept, and the same rescue, read through to its Finding in
// its panel and a Stash.

import { dialog, inside, role, walk } from "../walk";

const BAY = (n: number) => role("listitem", `slot-${n}`, { exact: true });
const TILE = (n: number) => inside(BAY(n), role("button", `slot-${n}`, { exact: true }));
const PANEL = dialog("slot-8");
const IN_PANEL = (kind: string, name: string) => inside(PANEL, role(kind, name, { exact: true }));

export const rescuingAKeptJobsSlot = walk("cleanup/slots", [
  { press: role("button", "Cleanup", { exact: true }), say: "Cleanup, from the rail" },
  { look: BAY(8), say: "A killed Job's slot: a solid card in the warning hue, with the reason it was kept" },
  { hover: inside(BAY(8), role("img", /^Kept/)), say: "Kept, and why" },
  { press: TILE(8), say: "Its panel opens" },
  { hover: IN_PANEL("button", "Rescue"), say: "Rescue, as on a stranded slot" },
  { press: IN_PANEL("button", "Rescue"), say: "A Scout starts reading it" },
  { look: inside(PANEL, role("heading", "Unfinished", { exact: true })), say: "Unfinished, and what is left" },
  { look: inside(PANEL, role("list", "Searched", { exact: true })), say: "What it read" },
  { hover: IN_PANEL("button", "Scrap"), say: "Scrap and Stash are in the panel" },
  { press: IN_PANEL("button", "Stash"), say: "Stashed, at once" },
  { look: inside(PANEL, role("status")), say: "The slot is free, and the commit is named" },
]);
