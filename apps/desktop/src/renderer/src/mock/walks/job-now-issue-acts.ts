// A stuck Drone and a failed Check, each with its own fix.

import { button, inside, region, role, walk } from "../walk";

const now = region("Now");

const w = walk("job-now-issue", [
  { look: inside(now, role("heading", "Issues")), say: "Issues" },
  { look: inside(now, role("img", "Drone stuck")), say: "A stuck Drone" },
  { look: inside(now, button("Redirect Drone")), say: "which can be redirected" },
  { look: inside(now, button("Retry step")), say: "or its step retried" },
  { look: inside(now, role("img", "Check failed")), say: "and a failed Check, with retry and skip" },
]);

export { w as "job-now-issue-acts" };
