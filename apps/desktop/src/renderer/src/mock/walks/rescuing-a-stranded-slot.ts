// A stranded slot on Cleanup rescued: a Scout reads it, its Finding opens on the
// bay, and the owner stashes it. A second stranded bay shows the Scrap's
// confirm, which names the files and unpushed commits that would go.

import { inside, role, walk } from "../walk";

const BAY = (n: number) => role("listitem", `slot-${n}`, { exact: true });
const ACT = (n: number, said: string) => inside(BAY(n), role("button", said, { exact: true }));

export const rescuingAStrandedSlot = walk("cleanup/slots", [
  { press: role("button", "Cleanup", { exact: true }), say: "Cleanup, from the rail" },
  { look: BAY(4), say: "A stranded slot: hatched, with its holder gone" },
  { hover: ACT(4, "Rescue"), say: "Rescue" },
  { press: ACT(4, "Rescue"), say: "A Scout starts reading it" },
  { look: inside(BAY(4), role("img", "Reading", { exact: true })), say: "Reading: the mark pulses, and the files appear as it goes" },
  { hover: ACT(4, "Stop"), say: "Stop ends the read" },
  { look: inside(BAY(4), role("list", "Searched", { exact: true })), say: "The Finding opens on the bay" },
  { hover: inside(BAY(4), role("img", "Uncommitted changes on top", { exact: true })), say: "Uncommitted changes were on top of the commit it read" },
  { hover: ACT(4, "Scrap"), say: "Scrap and Stash are offered on the Finding" },
  { hover: ACT(4, "Stash"), say: "Stash commits the uncommitted work to its branch" },
  { press: ACT(4, "Stash"), say: "Stashed, at once" },
  { look: inside(BAY(4), role("status")), say: "The slot is free, and the commit is named" },
  { press: ACT(7, "Rescue"), say: "The second stranded slot, rescued" },
  { look: inside(BAY(7), role("list", "Searched", { exact: true })), say: "Its Finding" },
  { press: ACT(7, "Scrap"), say: "Scrap" },
  { look: role("group", "Scrap slot-7", { exact: true }), say: "It names the uncommitted files and the unpushed commits, and nothing is sent until it is confirmed" },
]);
