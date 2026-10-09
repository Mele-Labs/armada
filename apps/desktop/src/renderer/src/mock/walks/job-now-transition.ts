// Nothing runs because the Job is between two steps, which the panel names.

import { inside, region, text, walk } from "../walk";

const now = region("Now");

const w = walk("job-now-transition", [
  { look: inside(now, text("Plan to Implement")), say: "The steps changing over, named" },
]);

export { w as "job-now-transition" };
