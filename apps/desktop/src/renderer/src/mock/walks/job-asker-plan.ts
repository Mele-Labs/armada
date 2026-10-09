// A Plan question: the Plan Drone's view while no option is hovered, and an option's sketch while one is.

import { inside, region, role, walk } from "../walk";

const now = region("Now");

const w = walk("job-asker-plan", [
  { look: role("group", "Asker", { exact: true }), say: "The Plan Drone, with the file it asks about marked" },
  { press: inside(now, role("radio", "Split it out")), say: "Pick an option" },
  { look: role("group", "Sketch", { exact: true }), say: "Its sketch takes the left" },
]);

export { w as "job-asker-plan" };
