// A Job that ended and kept its slot, beside the stranded ones: its own bay,
// the reason it was kept, and the same rescue, read through to its Finding in
// the side panel and a Stash.

import { dialog, inside, role, walk } from "../walk";

const BAY = (n: number) => role("listitem", `slot-${n}`, { exact: true });
const ACT = (n: number, said: string) => inside(BAY(n), role("button", said, { exact: true }));
const FINDING = dialog("Finding");
const IN_FINDING = (name: string) => inside(FINDING, role("button", name, { exact: true }));

export const rescuingAKeptJobsSlot = walk("cleanup/slots", [
  { press: role("button", "Cleanup", { exact: true }), say: "Cleanup, from the rail" },
  { look: BAY(8), say: "A killed Job's slot: a solid card in the warning hue, with the reason it was kept" },
  { hover: inside(BAY(8), role("img", /^Kept/)), say: "Kept, and why" },
  { hover: ACT(8, "Rescue"), say: "Rescue, as on a stranded slot" },
  { press: ACT(8, "Rescue"), say: "A Scout starts reading it" },
  { press: ACT(8, "Finding"), say: "The Finding opens in the side panel" },
  { look: inside(FINDING, role("list", "Searched", { exact: true })), say: "What it read" },
  { hover: IN_FINDING("Scrap"), say: "Scrap and Stash are in the panel" },
  { press: IN_FINDING("Stash"), say: "Stashed, at once" },
  { look: inside(BAY(8), role("status")), say: "The slot is free, and the commit is named" },
]);
