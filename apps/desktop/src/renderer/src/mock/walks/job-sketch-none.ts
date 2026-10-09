// An ask with no sketch leaves the canvas as it is.

import { card, inside, region, role, walk } from "../walk";

const w = walk("job-sketch-none", [
  { look: inside(region("Now"), role("img", "Asks you")), say: "The Drone asks, and it drew nothing" },
  { look: card("Write tests"), say: "The canvas stays, with no switch in the panel's head" },
]);

export { w as "job-sketch-none" };
