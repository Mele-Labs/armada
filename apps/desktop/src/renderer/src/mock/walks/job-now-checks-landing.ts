// Checks landing as they finish, each with retry and skip, and every Check skipped from the band.

import { region, button, inside, role, walk } from "../walk";

const now = region("Now");

const w = walk("job-now-checks", [
  { look: inside(now, role("img", "Check")), say: "Each row carries its kind as an icon" },
  { look: inside(now, role("img", "Passed")), say: "A Check that passed" },
  { look: inside(now, role("img", "Failed")), say: "and one that failed, each marked as it finished" },
  { look: inside(now, button("Retry now")), say: "A Check can be retried now" },
  { look: inside(now, button("Skip check")), say: "or skipped" },
  { look: inside(now, button("Skip all checks")), say: "and every Check can be skipped from the band" },
]);

export { w as "job-now-checks-landing" };
