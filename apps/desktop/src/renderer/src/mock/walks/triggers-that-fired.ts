// Triggers that fired in a Job (7 Oct 2026), a mock: one passed, one failed and has
// a repair Drone working on it. Each live state is a mark with a tooltip.

import { region, role, walk } from "../walk";

export const triggersThatFired = walk("real/job-2-at-review", [
  { look: region("Triggers"), say: "The triggers that fired, each with where it is set" },
  { hover: role("img", "Passed"), say: "One passed" },
  { hover: role("img", "Failed"), say: "One failed" },
  { hover: role("img", "Repair Drone working"), say: "A repair Drone is working on it, then the trigger goes again" },
]);
