// Nothing runs because the Job waits on other Jobs, each a row that opens it.

import { inside, region, role, walk } from "../walk";

const now = region("Now");

const w = walk("job-now-jobs", [
  { look: inside(now, role("heading", "Waiting")), say: "Waiting" },
  { look: inside(now, role("button", /Job Pin the store clock/)), say: "Each Job it waits on, a row that opens it" },
]);

export { w as "job-now-waiting-jobs" };
