// The approval canvas past the gate (prototype, 4 Oct 2026): the Overview for
// a Job's whole life, in its three lanes. Done nodes are marked done, the node
// it is at is marked live, Groups holds the plan's own, and a step, a group or
// a task opens the same panel it opens on Workflow and Plan, with the way back.

import { button, card, dialog, inside, region, role, text, walk } from "../walk";

const RUN = region("This Job's run");
const GROUP_3 = role("button", /^Group 3, /);

export const aRunningJobOnItsCanvas = walk("proto/feature-running", [
  { look: RUN, say: "The same lanes, now the Job's run: setup done, the work under way, delivery ahead" },
  { look: role("button", /^Plan the change, advanced/), say: "Plan is done" },
  { hover: inside(card("Implement"), text("running")), say: "Implement is where the Job is" },
  { look: inside(card("Implement"), text(/\d+[hms]/)), say: "and says how long it has run, as Workflow's card does" },
  { look: role("button", /^Group 1, passed/), say: "The plan's groups fan out under Implement: two passed" },
  { hover: inside(GROUP_3, text("running")), say: "and the third at work" },
  { press: card("Implement"), say: "A step opens its panel, Workflow's own" },
  { look: button("Back to Overview"), say: "with the way back to the run" },
  { press: button("Back to Overview"), say: "Back" },
  { press: GROUP_3, say: "A group opens Plan's panel" },
  { press: role("button", /Answer restart and move in the mock/), say: "and a task from it" },
  { look: text("Answer restart and move in the mock"), say: "T4, its own panel" },
  { press: button("Back to Group 3"), say: "Back to its group, as on Plan" },
  { press: button("Back to Overview"), say: "and back to the run" },
  { press: card("Land"), say: "Land has no panel on another tab: its card" },
  { look: inside(dialog("Land"), role("combobox", "Lands in")), say: "No branch named: pick one" },
  { look: inside(dialog("Land"), text("The work is delivered")), say: "Complete when" },
]);
