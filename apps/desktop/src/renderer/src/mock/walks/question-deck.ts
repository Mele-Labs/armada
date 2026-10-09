// Four questions as a deck: one in front and the rest standing behind it. Picking an answer brings
// the next one forward; what is answered sits above as chips; the last one sends. Over
// `session-many-questions`.

import { kit } from "../sessions/walk-kit";
import { button, inside, region, role, text, walk } from "../walk";

const { rail } = kit(false);
const ask = role("article", "Waiting on you");

const questionDeck = walk("session-many-questions", [
  { press: rail("Sessions"), say: "The Sessions page" },
  { press: inside(region("Sessions"), button(/Plan the night/)), say: "A Session asking four questions" },
  { look: inside(ask, text("Should a destructive best answer be decided overnight?")), say: "One question in front, three behind" },
  { press: inside(ask, role("radio", "Hold for me")), say: "Picking an answer" },
  { look: inside(ask, text("Should the Morning review open when Sleep goes off?")), say: "Brings the next one forward" },
  { press: inside(ask, role("radio", "Only if non-empty")), say: "And the next" },
  { press: inside(ask, role("radio", "Left of Helm")), say: "And the next" },
  { look: inside(ask, role("group", "Answered")), say: "What is answered sits above; press one to change it" },
  { press: inside(ask, role("radio", "Approve all")), say: "The last one" },
  { look: inside(ask, button("Answer")), say: "Sends them all" },
]);

export { questionDeck };
