// A running step's phase track, with its Drone the part now, on its task: the
// Drones lit and moving, the Checks and the Judge next. Along the bottom of its
// Workflow card and in its panel's header. The owner's annotation of
// 3 Oct 2026, `ouqa`; `step-phase.test.tsx` holds the claims.

import { button, inside, role, tab, walk } from "../walk";

const card = button(/^Implement, running, Drones? on T\d+/);
const drones = role("img", /^Drones? on T\d+/);

export const aStepWithADroneOnIt = walk("arc/executing-sequential", [
  { press: tab("Workflow"), say: "The run, top to bottom" },
  { hover: inside(card, drones), say: "Its Drone is at work now, and on which task" },
  { hover: inside(card, role("img", "Checks next")), say: "The Checks are next" },
  { press: card, say: "Its panel" },
  { hover: inside(role("dialog", "Implement"), drones), say: "The same track, under running" },
]);
