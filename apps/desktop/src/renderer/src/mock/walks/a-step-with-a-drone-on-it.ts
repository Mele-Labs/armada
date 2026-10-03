// A running step whose Drone is at work says so, and on which task: a mark
// beside the step's own on its Workflow card, and in its panel. The owner's
// annotation of 3 Oct 2026, `ouqa`; `step-phase.test.tsx` holds the claims.

import { button, inside, role, tab, walk } from "../walk";

const card = button(/^Implement, running, Drone on T\d+$/);
const drone = role("img", /^Drone on T\d+$/);

export const aStepWithADroneOnIt = walk("arc/executing-sequential", [
  { press: tab("Workflow"), say: "The run, top to bottom" },
  { hover: inside(card, drone), say: "Implement's Drone, and the task it is on" },
  { press: card, say: "Its panel" },
  { hover: inside(role("dialog", "Implement"), drone), say: "The same mark, beside running" },
]);
