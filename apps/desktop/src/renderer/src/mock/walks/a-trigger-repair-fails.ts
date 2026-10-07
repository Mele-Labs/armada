// The last end of a repair (7 Oct 2026): the Drone finds no fix in either try, the branch ends in a
// failed mark, and the Job gets an alert.

import { inside, region, role, walk } from "../walk";

const RUN = region("This Job's run");

export const aTriggerRepairFails = walk("real/job-2-trigger-repair-fails", [
  { look: inside(RUN, role("img", "Repair Drone working")), say: "The repair Drone works on the failed trigger" },
  { look: role("img", "Alert"), say: "Neither try finds a fix: the Job gets an alert" },
  { look: inside(RUN, role("img", "Failed")), say: "The branch ends in a failed mark" },
  { look: RUN, say: "On the Job's run" },
]);
