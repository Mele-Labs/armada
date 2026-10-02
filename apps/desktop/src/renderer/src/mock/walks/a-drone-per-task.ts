// A plan worked one task at a time, each by a Drone of its own: spike 022,
// slice 1b (protocol 23.1). Recorded off a dev Fleet, so every value is what
// Fleet served. Each state is a mark, named on hover.

import { card, row, tab, text, walk } from "../walk";

const aDronePerTask = walk("recorded/drone-per-task", [
  { press: tab("Plan"), say: "Five tasks, each marked done once the step's Checks passed" },
  { press: card("Write file 3"), say: "One task, opened" },
  { look: text("task-T3.txt holds it"), say: "What its own Drone handed in as showing it done" },
  { press: tab("Drones"), say: "Every Drone the Job had: the plan's, one per task, and the last step's" },
  { look: row("Drone on T1"), say: "T1's own Drone, done once it handed T1 in" },
  { look: row("Drone on T5"), say: "One Drone per task, spawned in plan order" },
]);

// Named as the link spells it, `?walk=a-drone-per-task`.
export { aDronePerTask as "a-drone-per-task" };
