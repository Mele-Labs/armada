// A running step says which part of itself it is in: here, its gate's Checks.
// A mark beside the step's own on its Workflow card, and on the state pill in
// its panel, named by its tooltip. The owner's annotation of 3 Oct 2026,
// `ouqa`; `step-phase.test.tsx` holds the claims.

import { button, inside, role, tab, walk } from "../walk";

const checks = role("img", "Checks running");

export const aStepRunningItsChecks = walk("check-logs", [
  { press: tab("Workflow"), say: "The run, top to bottom" },
  { hover: inside(button("Implement, running, Checks running"), checks), say: "Implement is running its Checks" },
  { press: button("Implement, running, Checks running"), say: "Its panel" },
  { hover: inside(role("dialog", "Implement"), checks), say: "The same mark, beside running" },
]);
