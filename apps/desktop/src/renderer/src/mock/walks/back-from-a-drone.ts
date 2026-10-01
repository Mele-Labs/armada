// A Drone opened from a step's panel leaves a way back to the step. The
// owner's decision of 29 Sep 2026; `job-detail-workflow.test.tsx` holds the claim.

import { button, card, tab, walk } from "../walk";

export const backFromADrone = walk("arc/executing-sequential", [
  { press: tab("Workflow"), say: "The run, top to bottom" },
  { press: card("Implement"), say: "Its panel lists every Drone on the step" },
  { press: button("Drone on T1"), say: "A Drone opens in Drones" },
  { look: button("Back to Implement"), say: "The way back is in its head" },
  { press: button("Back to Implement"), say: "Back lands on the step, panel open" },
]);
