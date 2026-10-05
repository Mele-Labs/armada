// A Job that ended and kept its slot, beside the stranded ones: its own bay,
// the reason it was kept, and the same rescue, read through to its Finding and
// a Stash.

import { inside, role, walk } from "../walk";

const BAY = (n: number) => role("listitem", `slot-${n}`, { exact: true });
const ACT = (n: number, said: string) => inside(BAY(n), role("button", said, { exact: true }));

export const rescuingAKeptJobsSlot = walk("cleanup/slots", [
  { press: role("button", "Cleanup", { exact: true }), say: "Cleanup, from the rail" },
  { look: BAY(8), say: "A killed Job's slot: a solid card in the warning hue, with the reason it was kept" },
  { hover: inside(BAY(8), role("img", /^Kept/)), say: "Kept, and why" },
  { hover: ACT(8, "Rescue"), say: "Rescue, as on a stranded slot" },
  { press: ACT(8, "Rescue"), say: "A Scout starts reading it" },
  { look: inside(BAY(8), role("list", "Searched", { exact: true })), say: "The Finding opens on the bay" },
  { hover: ACT(8, "Scrap"), say: "Scrap and Stash are offered" },
  { press: ACT(8, "Stash"), say: "Stashed, at once" },
  { look: inside(BAY(8), role("status")), say: "The slot is free, and the commit is named" },
]);
