// Run Checks again pressed on a Job out of retries. The step still reads
// stopped and Fleet offers nothing while the run is out, so Overview's lead
// says the Checks are running and names the one running now, with no act; the
// Workflow card and the step panel draw the phase track with Checks the part
// now. The owner's Job 3, 4 Oct 2026; `lead.test.ts` and `step-phase.test.ts`
// hold the claims.

import { card, inside, role, tab, text, walk } from "../walk";

const regression = card("Regression check");
const panel = role("dialog", "Regression check");

export const checksRunningAgain = walk("repair/checks-again", [
  { look: role("heading", "Running Checks again"), say: "The Checks are running again" },
  { look: text("desktop_test"), say: "The Check running now" },
  { press: tab("Workflow"), say: "The run, top to bottom" },
  { hover: inside(regression, role("img", "Checks running")), say: "The stopped step's Checks, running now" },
  { press: regression, say: "Its panel" },
  { hover: inside(panel, role("img", "Checks running")), say: "The same track, under stopped" },
]);
