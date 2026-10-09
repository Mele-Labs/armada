// A Trigger on a destructive Command asks the owner first (8 Oct 2026): `wipe_qa` waits on him, and since it
// blocks, the Job's gate waits too. The Board row rings a bell; the Job's canvas draws the Trigger as a leaf
// with Run and Skip. Run runs the Command once, and it passes. Skip is the other answer, and records the
// firing skipped by him.

import { button, inside, region, role, walk } from "../walk";

const RUN = region("This Job's run");

export const aDestructiveTriggerAsks = walk("real/job-2-trigger-asks", [
  { look: region(/^Job:/), say: "A call comes forward over the Dashboard: a Trigger waits on him" },
  { press: button(/^Open Job/), say: "The Job" },
  { look: inside(RUN, role("group", /wipe_qa, PR opened/)), say: "The Trigger, as a leaf off the step it fired at" },
  { hover: inside(RUN, role("img", "Waiting on you")), say: "A destructive Command is not run until he says so" },
  { look: inside(RUN, button("Skip")), say: "Skip lets it go and records it skipped by him" },
  { press: inside(RUN, button("Run")), say: "Run" },
  { look: inside(RUN, role("img", "Passed")), say: "The Command ran once and passed" },
]);
