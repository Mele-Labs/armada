// A failed Trigger with Self repair (7 Oct 2026): `deploy_qa` fails after the PR opens and a branch
// grows off the workflow at that point. The repair Drone works on it, then the branch holds with the
// fix and asks where it goes: on this branch, or as a PR of its own. A PR of its own passes the
// Trigger at once, the Command having passed on the repair branch. The Workflow tab draws the same
// branch. Past the gate the Overview canvas is the Job's run. Every row is shaped as Fleet serves it.

import { button, inside, region, role, tab, text, walk } from "../walk";

const RUN = region("This Job's run");

export const aTriggerRepairBranch = walk("real/job-2-trigger-repair", [
  { look: RUN, say: "A Job past PR open, on its canvas" },
  { look: inside(RUN, role("img", "Repair branch")), say: "A branch off the workflow where deploy_qa fired" },
  { hover: inside(RUN, role("img", "Repair Drone working")), say: "The repair Drone, working" },
  { look: inside(RUN, role("group", "Where the fix goes")), say: "The fix is found: the branch holds and asks" },
  { look: inside(RUN, role("list", "The fix")), say: "What the fix changes, beside the choice" },
  { press: inside(RUN, button("New PR")), say: "New PR" },
  { look: inside(RUN, role("img", "New PR")), say: "The branch ends in a PR of its own" },
  { look: inside(RUN, text("#1751")), say: "With the PR's number" },
  { look: inside(RUN, role("img", "Passed")), say: "And the trigger passes" },
  { press: tab("Workflow"), say: "The Workflow tab" },
  { look: role("img", "New PR"), say: "The same branch off the step that delivers" },
]);
