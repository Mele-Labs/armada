// Triggers as leaves (8 Oct 2026): every Trigger a Job holds is a leaf off the step it fired at, inside
// that step's lane, and the lane grows to hold them. A passed one, a repair's branch and a Drone's
// branch stand the same way, the stacked run offers the same acts, and the Workflow editor draws a
// Trigger set for every workflow as a leaf on each workflow's canvas.

import { button, card, inside, region, role, tab, text, walk } from "../walk";

const RUN = region("This Job's run");

export const triggersAsLeaves = walk("real/job-2-leaves", [
  { look: RUN, say: "A Job past PR open: each Trigger is a leaf off the step it fired at" },
  { hover: inside(RUN, role("img", "Passed")), say: "A Trigger that passed, beside the step it fired at" },
  { look: inside(RUN, role("img", "Repair branch")), say: "A repair's branch, inside the lane of the step it grew from" },
  { look: inside(RUN, role("group", "Where the fix goes")), say: "Its choice, on the leaf" },
  { hover: inside(RUN, role("img", "Drone branch")), say: "A Skill's Drone and an added step's Drone stand the same way" },
  { press: tab("Workflow"), say: "The Workflow tab" },
  { press: tab("Stacked"), say: "The list" },
  { look: role("group", "Where the fix goes"), say: "The list offers the same choice" },
  { look: button("This branch"), say: "This branch" },
  { look: button("New PR"), say: "New PR" },
  { look: button("Rerun"), say: "And Rerun and Skip on a hold" },
  { press: button("Workflows", { exact: true }), say: "Workflows, from the rail" },
  { press: button("feature, carried"), say: "The Feature workflow" },
  { hover: role("img", "Every workflow"), say: "A Trigger set for every workflow is a leaf off the step it fires at, marked" },
  { look: inside(card("Implement"), text("On pass")), say: "A Trigger set for this workflow alone is a line on the step" },
]);
