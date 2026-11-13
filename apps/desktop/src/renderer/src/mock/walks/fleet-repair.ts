// The one button the notice over Fleet offers, which depends on which side is stale. A Fleet on an older kept build
// than the current one offers Restart Fleet, and this Bridge on an older one offers Reopen Armada. Over
// `scenarios/fleet-repair.ts`, which moves from one to the next as the walk goes on. Start Fleet, for no Fleet
// running, has no walk: see that file.

import { button, text, walk } from "../walk";

export const fleetRepair = walk("fleet/repair", [
  {
    look: text("Fleet and Bridge do not match"),
    say: "The notice names both sides",
  },
  {
    hover: button("Restart Fleet"),
    say: "Fleet is the stale side, so the button restarts it. The tooltip says what a working Drone costs",
  },
  {
    later: button("Restart Fleet"),
    say: "Fleet is on the current build now, and this Bridge is on an older one",
  },
  {
    hover: button("Reopen Armada"),
    say: "Bridge is the stale side, so the button reopens it. The tooltip says Fleet is not touched",
  },
]);
