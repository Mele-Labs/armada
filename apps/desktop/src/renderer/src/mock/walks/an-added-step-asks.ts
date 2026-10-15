// A Script added to one Job on a destructive Command asks the owner first, as a saved Trigger does
// (8 Oct 2026): `wipe_qa` waits on him, and since it blocks, the Job's gate waits too. The call is in Incoming
// calls; the Job's canvas draws the step as a leaf with Run and Skip. Run runs the Command once, and it passes.

import { button, card, inside, region, role, tab, walk } from "../walk";

const LEAF = role("group", /wipe_qa, PR opened/);

export const anAddedStepAsks = walk("real/job-2-added-step-asks", [
  { look: inside(region("Incoming calls"), button(/^Job: /)), say: "A call arrives: a step added to this Job waits on him" },
  { press: inside(region("Incoming calls"), button(/^Job: /)), say: "Pressing it opens the Job" },
  { press: button("Open the Job page"), say: "Open the Job page: the leaf's acts are on the page itself" },
  { press: tab("Workflow"), say: "Its workflow, on the canvas" },
  { look: LEAF, say: "The added step, as a leaf off the step it fired at" },
  { hover: inside(LEAF, role("img", "Waiting on you")), say: "A destructive Command is not run until he says so" },
  { look: inside(LEAF, button("Skip")), say: "Skip lets it go and records it skipped by him" },
  { press: inside(LEAF, button("Run")), say: "Run" },
  { look: card("wipe_qa"), say: "The Command ran once and passed, and the step is done" },
]);
