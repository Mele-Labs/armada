// A Job the gaming check holds is answered under Overview's lead and in the
// Workflow step panel, with one block. The owner's decision of 2 Oct 2026,
// #1672; `job-detail.test.tsx` holds the claims.

import { button, card, inside, region, role, tab, text, walk } from "../walk";

const asks = region("Question for you");

export const aJobHeldByTheGamingCheck = walk("held/gaming-check", [
  { look: text("3 commands were refused during Regression check"), say: "The lead names what the Drone did" },
  { look: text(/^An assertion was removed or loosened/), say: "What happened, and that it was judged to weaken coverage" },
  { look: text("The gaming check asked:"), say: "Who asked, in plain words, with the brief one press away" },
  { look: role("textbox", "Note (optional)"), say: "One note, sent with whichever answer you press" },
  { look: button("Carry on"), say: "Thumbs up: the work is fine, and the Job goes on" },
  { look: button("Send it back"), say: "Thumbs down, in red: the Drone still on the step gets the flag" },
  { press: tab("Workflow"), say: "The run, top to bottom" },
  { press: card("Regression check"), say: "The held step's panel" },
  { look: inside(asks, role("group", "Answer the flag")), say: "The same block, first in the panel" },
]);
