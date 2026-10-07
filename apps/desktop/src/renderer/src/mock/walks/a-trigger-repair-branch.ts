// A failed trigger with Self repair on (7 Oct 2026), a mock: `deploy_qa` fails
// after the PR opens and a branch grows off the workflow at that point. The
// repair Drone works on it, then the branch holds with the fix and asks where
// it goes: on this branch, or as a PR of its own. The trigger then runs again.
// The Workflow tab draws the same branch. Past the gate the Overview canvas is
// the Job's run.

import { button, inside, region, role, tab, walk } from "../walk";

const RUN = region("This Job's run");

export const aTriggerRepairBranch = walk("real/job-2-at-review", [
  { look: RUN, say: "A Job past PR open, on its canvas" },
  { look: inside(RUN, role("img", "Repair branch")), say: "A branch off the workflow where deploy_qa fired" },
  { hover: inside(RUN, role("img", "Repair Drone working")), say: "The repair Drone, working" },
  { look: inside(RUN, role("group", "Where the fix goes")), say: "The fix is found: the branch holds and asks" },
  { look: inside(RUN, role("list", "The fix")), say: "What the fix changes, beside the choice" },
  { press: inside(RUN, button("New PR")), say: "New PR" },
  { look: inside(RUN, role("img", "Trigger running again")), say: "The trigger runs again" },
  { look: inside(RUN, role("img", "New PR")), say: "The branch ends in a PR of its own" },
  { look: inside(RUN, role("img", "Passed")), say: "And the trigger passes" },
  { press: tab("Workflow"), say: "The Workflow tab" },
  { look: role("img", "New PR"), say: "The same branch off the step that delivers" },
]);
