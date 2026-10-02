// The owner's Job 2, merged, as Fleet served it: its groups one row each,
// nothing drawn over the next, and the commit short with the whole of it on
// hover. No task count, no "not timed", no line about what nothing timed.

import { role, text, walk } from "../walk";

export const groupsOnALandedJob = walk("real/job-2-landed", [
  { look: role("list", "Groups"), say: "One row per group, nothing drawn over the next" },
  { look: text("daae542"), say: "The commit, short; hover it for the whole of it" },
]);
