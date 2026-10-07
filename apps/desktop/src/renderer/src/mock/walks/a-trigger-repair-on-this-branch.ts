// The other end of a repair (7 Oct 2026), a mock: the fix goes on this branch,
// the Job's open PR, and the repair branch joins the spine again.

import { button, inside, region, role, walk } from "../walk";

const RUN = region("This Job's run");

export const aTriggerRepairOnThisBranch = walk("real/job-2-at-review", [
  { look: inside(RUN, role("group", "Where the fix goes")), say: "The fix is found, and the branch asks" },
  { press: inside(RUN, button("This branch")), say: "This branch" },
  { look: inside(RUN, role("img", "Passed")), say: "The trigger runs again and passes" },
]);
