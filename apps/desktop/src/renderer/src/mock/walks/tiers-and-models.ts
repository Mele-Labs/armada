// A plan whose tasks each ran on the model their tier, or a person, picked
// (spike 022, slice 3): each Drone names the model it ran, each row the model
// its task runs on, a tier the Job's map leaves out names none, and Edit this
// task offers a model of the person's own. `plan-tiers-from-fleet.test.tsx`
// holds the claims, Save included.

import { button, inside, role, tab, walk } from "../walk";

const GROUPS = role("list", "Groups, in the order they run");
const t3 = inside(GROUPS, role("listitem", /^T3 /));
const t4 = inside(GROUPS, role("listitem", /^T4 /));

export const tiersAndModels = walk("real/tiers-and-models", [
  { press: tab("Drones"), say: "Every Drone, and the model each one ran" },
  { look: role("columnheader", "Model"), say: "T1's on opus, T2's on haiku, T3's and T4's on sonnet" },
  { press: tab("Plan"), say: "The plan, each task with the model it runs on" },
  { press: tab("List"), say: "T1 is difficult and runs on opus, T2 easy on haiku" },
  { look: t3, say: "T3 has no tier: Armada picks, so no model is drawn" },
  { look: t4, say: "T4 failed, and a person picked opus for its next Drone" },
  { press: inside(t4, button(/Answer restart and move/)), say: "Its panel: medium, and the model picked over the map" },
  { press: button("Edit this task"), say: "Edit this task, after the failure" },
  { look: role("combobox", "Model"), say: "Any model on offer, over the map. Save keeps it on the task" },
]);
