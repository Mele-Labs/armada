// A Job out of retries on a failed `diff_nonempty`, with the override offered
// as the third act: the dialog names the Check and takes a reason, and the
// step advances recorded as failed. Job 12, 6 Oct 2026; `StepActs.test.tsx`
// and `Overrule.test.tsx` hold the claims.

import { button, dialog, inside, role, text, walk } from "../walk";

const overrule = "Overrule diff_nonempty";
const sheet = dialog("Overrule diff_nonempty on this step?");

export const overrideACheck = walk("repair/override-a-check", [
  { look: role("heading", "Out of retries"), say: "The step spent its retries on a failed Check" },
  { look: button("Run Checks again"), say: "Run the Checks again on the work here" },
  { look: button("Restart step"), say: "Restart the step on the same worktree" },
  { hover: button(overrule), say: "The third act goes past the Check, and says which one" },
  { press: button(overrule), say: "It asks for a reason" },
  { look: sheet, say: "The Check is named, and what it costs" },
  { type: "The work is done: the change is on the branch and the Check read it before it landed", into: role("textbox", "Why this step is done"), say: "The reason goes on the Job's log" },
  { press: inside(sheet, button(overrule)), say: "Confirm" },
  { look: text(/Check the consumers still compile/), say: "The step advanced, recorded as failed, and the next one runs" },
]);
