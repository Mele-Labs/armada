// A Job the gaming check holds is answered under Overview's lead and in the
// Workflow step panel, with one block. The owner's decision of 2 Oct 2026,
// #1672; `job-detail.test.tsx` holds the claims.

import { button, card, inside, region, role, tab, text, walk } from "../walk";

const asks = region("Question for you");

export const aJobHeldByTheGamingCheck = walk("held/gaming-check", [
  { look: text("3 commands were refused during Regression check"), say: "The lead names what the Drone did" },
  { look: text(/^An assertion now asserts less ·/), say: "And what the gaming check caught" },
  { look: role("group", "Is the flag right?"), say: "Answered under the lead: Carry on, or Send it back" },
  { look: button("Send it back"), say: "The Drone is still on the step, so this redirects it with the flag" },
  { press: tab("Workflow"), say: "The run, top to bottom" },
  { press: card("Regression check"), say: "The held step's panel" },
  { look: inside(asks, role("group", "Is the flag right?")), say: "The same block, first in the panel" },
]);
