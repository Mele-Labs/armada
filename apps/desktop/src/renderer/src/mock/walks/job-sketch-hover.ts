// Hovering a Plan option previews the sketch of how the result looks if it is taken.

import { inside, region, role, walk } from "../walk";

const now = region("Now");

const w = walk("job-sketch-plan", [
  { look: role("img", "Sketch"), say: "With nothing hovered, the decision's own sketch: how it is now" },
  { hover: inside(now, role("radio", "Split it out")), say: "Hover an option and its sketch stands in" },
  { hover: inside(now, role("radio", "Wrap it in place")), say: "Another option, another sketch" },
]);

export { w as "job-sketch-hover" };
