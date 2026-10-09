// The canvas keeps the steps the panel's rows belong to lit and stands every other node back; hiding the panel lets go.

import { button, card, inside, region, role, walk } from "../walk";

const now = region("Now");

const w = walk("job-now-drone", [
  { look: inside(now, role("heading", "Running")), say: "The Drone belongs to Implement" },
  { look: card("Write tests"), say: "Every other step stands back, its Checks and Judge with it" },
  { press: inside(now, button("Hide now")), say: "Hide the panel" },
  { press: button("Show now"), say: "and the canvas lets go. Showing it focuses again" },
]);

export { w as "job-now-canvas-focus" };
