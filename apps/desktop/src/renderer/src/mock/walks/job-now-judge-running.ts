// A Judge reading, as a row under Judges.

import { inside, region, role, text, walk } from "../walk";

const now = region("Now");

const w = walk("job-now-judge", [
  { look: inside(now, text("Judges")), say: "Judges" },
  { look: inside(now, role("button", /Judge on Implement, .*running/)), say: "The Judge, running" },
]);

export { w as "job-now-judge-running" };
