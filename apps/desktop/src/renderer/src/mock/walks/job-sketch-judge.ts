// A Judge's question can carry a sketch, shown where the canvas was.

import { inside, region, role, walk } from "../walk";

const w = walk("job-sketch-judge", [
  { look: inside(region("Now"), role("img", "Asks you")), say: "The Judge asks, and its sketch stands where the canvas was" },
  { look: role("group", "Sketch", { exact: true }), say: "Waiting, Retrying, Capped, Failed" },
]);

export { w as "job-sketch-judge" };
