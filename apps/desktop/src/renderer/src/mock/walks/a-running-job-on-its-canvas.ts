// The approval canvas past the gate (prototype, 4 Oct 2026): the Overview for
// a Job's whole life. Done nodes are marked done, the node it is at is marked
// live, Groups holds the plan's own, and every card reads.

import { card, dialog, inside, region, role, text, walk } from "../walk";

const RUN = region("This Job's run");
const GROUP_3 = role("group", /^Group 3, /);

export const aRunningJobOnItsCanvas = walk("proto/feature-running", [
  { look: RUN, say: "The same canvas, now the Job's run" },
  { look: role("button", /^Plan the change, advanced/), say: "Plan is done" },
  { hover: inside(card("Implement"), text("running")), say: "Implement is where the Job is" },
  { look: role("group", /^Group 1, passed/), say: "Groups holds the plan's own: two passed" },
  { hover: inside(GROUP_3, text("running")), say: "and the third at work" },
  { press: card("Plan the change"), say: "A card past the gate" },
  { look: inside(dialog("Plan the change"), text("Auto")), say: "reads what it ran on" },
  { press: card("Land"), say: "Where it lands" },
  { look: inside(dialog("Land"), role("combobox", "Lands in")), say: "No branch named: pick one" },
  { look: inside(dialog("Land"), text("The work is delivered")), say: "Complete when" },
]);
