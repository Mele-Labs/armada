// Nothing runs because the Job waits on a resource, which the panel names.

import { inside, region, role, text, walk } from "../walk";

const now = region("Now");

const w = walk("job-now-resource", [
  { look: inside(now, role("heading", "Waiting")), say: "Waiting" },
  { look: inside(now, text("Worktree slot")), say: "The resource, named" },
]);

export { w as "job-now-waiting-resource" };
