// A plan worked one task at a time, each by a Drone of its own: spike 022,
// slice 1b (protocol 23.1). Recorded off a dev Fleet, so every value is what
// Fleet served. Each state is a mark, named on hover. The task's panel lists
// its Drones (the owner, 2 Oct 2026), and opens one where the Drones tab does.

import { button, card, dialog, inside, role, row, tab, text, walk } from "../walk";

const sheet = dialog("Write file 3");
const itsDrones = role("list", "Drones on this task");

const aDronePerTask = walk("recorded/drone-per-task", [
  { press: tab("Plan"), say: "Five tasks, each marked done once the step's Checks passed" },
  { press: card("Write file 3"), say: "One task, opened" },
  { look: inside(sheet, role("img", "Done")), say: "Its state is the mark alone, named on hover" },
  { look: text("task-T3.txt holds it"), say: "What its own Drone handed in as showing it done" },
  { look: itsDrones, say: "The Drone that worked T3: its state mark, turns and cost" },
  { press: inside(itsDrones, button("Drone on T3")), say: "It opens in Drones, as a row there does" },
  { look: dialog("Drone on T3"), say: "T3's own Drone, read whole" },
  { look: row("Drone on T5"), say: "One Drone per task, spawned in plan order" },
]);

// Named as the link spells it, `?walk=a-drone-per-task`.
export { aDronePerTask as "a-drone-per-task" };
