// A Check row opens that Check's log sheet.

import { inside, region, role, text, walk } from "../walk";

const now = region("Now");

const w = walk("job-now-checks", [
  { press: inside(now, role("button", /Open lint, failed/)), say: "A press opens the Check" },
  { look: text("Check log"), say: "Its log sheet" },
]);

export { w as "job-now-check-log" };
