// An issue read in lands inside one Zone, with one line to it from the issue,
// and each Cluster drawn round its Notes; dragging the Zone carries all of it,
// and a Zone is put on the board from the rail. The owner's read of #1657 on
// 2 Oct 2026, landed the way #1620 now lands one. `studio-zone.test.tsx` holds
// the claims.

import { button, role, walk } from "../walk";

const zone = role("group", "Zone", { exact: true });

export const aReadInLandsInAZone = walk("studio-zone", [
  { press: button("Studios", { exact: true }), say: "This repository's Studios" },
  { press: button("Open", { exact: true }), say: "The Studio the issue was read into" },
  { press: button("Continue", { exact: true }), say: "Continue it, so it can be moved" },
  { look: zone, say: "Everything the read-in brought back sits in one Zone, with one line to it from the issue" },
  { look: role("group", "Cluster: The problem today"), say: "Each Cluster is a box round its own Notes" },
  {
    look: role("group", /^Note: Watch for a `scope` change/),
    say: "A Note sits inside the Cluster it was read into, Risks to watch",
  },
  { drag: zone, by: { x: 0, y: 160 }, say: "Drag the Zone, and everything in it comes along" },
  { look: role("group", "Cluster: What to build"), say: "Still inside, where it was" },
  { press: button("Add a Zone"), say: "Add a zone, from the rail. Press the board to put it down" },
]);
