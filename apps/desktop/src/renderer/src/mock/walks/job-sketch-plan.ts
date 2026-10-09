// A Plan decision with a sketch shows it where the canvas was, and Next shows the next decision's.

import { button, card, inside, region, role, walk } from "../walk";

const now = region("Now");

const w = walk("job-sketch-plan", [
  { look: role("group", "Sketch", { exact: true }), say: "The first decision's sketch stands where the canvas was" },
  { press: inside(now, role("radio", "Wrap it in place")), say: "Pick an answer" },
  { press: inside(now, button("Next")), say: "Next brings the second decision's sketch" },
  { press: inside(now, role("radio", "Add a fake clock")), say: "Pick" },
  { press: inside(now, button("Next")), say: "The third decision has no sketch, so the canvas is back" },
  { look: card("Write tests"), say: "The canvas, steps and all" },
]);

export { w as "job-sketch-plan" };
