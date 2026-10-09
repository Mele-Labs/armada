// Nothing runs but a one-off step, which the panel shows.

import { inside, region, text, walk } from "../walk";

const now = region("Now");

const w = walk("job-now-one-off", [
  { look: inside(now, text("Handoff, a one-off step")), say: "The step, shown" },
]);

export { w as "job-now-one-off-step" };
