// The owner can switch from the sketch to the canvas and back while the ask is open.

import { button, card, inside, region, role, walk } from "../walk";

const now = region("Now");

const w = walk("job-sketch-judge", [
  { look: role("img", "Sketch"), say: "The sketch shows while the ask is open" },
  { press: inside(now, button("Canvas")), say: "Canvas puts the canvas back" },
  { look: card("Write tests"), say: "Steps and all, the ask still open" },
  { press: inside(now, button("Sketch")), say: "Sketch brings the drawing back" },
  { look: role("img", "Sketch"), say: "Drawn where the canvas was" },
]);

export { w as "job-sketch-canvas" };
