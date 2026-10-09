// A Judge reading, as a row under Judges.

import { region, inside, role, walk } from "../walk";

const now = region("Now");

const w = walk("job-now-judge", [
  { look: inside(now, role("img", "Judge")), say: "The row's kind is its icon" },
  { look: inside(now, role("button", /Judge on Implement, .*running/)), say: "The Judge, running" },
]);

export { w as "job-now-judge-running" };
