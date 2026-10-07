// A blocking Trigger that fails holds the Job (7 Oct 2026): `deploy_qa` has `block` on and failed after the
// PR opened, so the gate waits on the owner. The hold is drawn where it fired, with Rerun and Skip. Rerun
// runs the Command again and the hold lets go. `aTriggerHoldSkipped` is the other ending.

import { button, inside, region, role, walk } from "../walk";

const RUN = region("This Job's run");

export const aTriggerHoldsTheJob = walk("real/job-2-trigger-hold", [
  { look: inside(RUN, role("group", /deploy_qa, PR opened/)), say: "A Trigger holds the Job, where it fired" },
  { hover: inside(RUN, role("img", /Held, deploy_qa/)), say: "The Trigger and its moment" },
  { look: role("img", "Alert"), say: "The Job's alert" },
  { press: inside(RUN, button("Rerun")), say: "Rerun" },
  { look: role("img", "Passed"), say: "The Command passes and the hold lets go" },
]);

export const aTriggerHoldSkipped = walk("real/job-2-trigger-hold-skipped", [
  { look: inside(RUN, role("group", /deploy_qa, PR opened/)), say: "A Trigger holds the Job" },
  { press: inside(RUN, button("Skip")), say: "Skip" },
  { look: role("img", /Skipped by the owner|was skipped by the owner/), say: "The firing is skipped, by the owner" },
]);
