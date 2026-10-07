// The other end of a repair (7 Oct 2026): the fix goes on this branch, the Job's open PR. Fleet merges
// it, pushes it and runs the Command again on the Job's branch, and the repair branch joins the spine.

import { button, inside, region, role, walk } from "../walk";

const RUN = region("This Job's run");

export const aTriggerRepairOnThisBranch = walk("real/job-2-trigger-repair", [
  { look: inside(RUN, role("group", "Where the fix goes")), say: "The fix is found, and the branch asks" },
  { press: inside(RUN, button("This branch")), say: "This branch" },
  { look: inside(RUN, role("img", "Trigger running again")), say: "The trigger runs again on the Job's branch" },
  { look: inside(RUN, role("img", "Passed")), say: "And passes" },
]);
