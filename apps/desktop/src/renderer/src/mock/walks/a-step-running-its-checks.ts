// A running step's phase track, with its gate's Checks the part now: the
// Drones done and the Checks lit and moving. This step declares no Judge, so
// the track draws none. On its Workflow card and in its panel's header. The
// owner's annotation of 3 Oct 2026, `ouqa`, and the track he drew on walking
// the first answer; `step-phase.test.tsx` holds the claims.

import { button, inside, role, tab, walk } from "../walk";

const card = button("Implement, running, Checks running");
const panel = role("dialog", "Implement");

export const aStepRunningItsChecks = walk("check-logs", [
  { press: tab("Workflow"), say: "The run, top to bottom" },
  { hover: inside(card, role("img", "Drones done")), say: "Its Drones are done" },
  { hover: inside(card, role("img", "Checks running")), say: "Its Checks are running now" },
  { press: card, say: "Its panel" },
  { hover: inside(panel, role("img", "Checks running")), say: "The same track, under running" },
]);
