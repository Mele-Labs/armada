// Four questions as a deck: one in front and the rest standing behind it. Picking an answer brings
// the next one forward; what is answered sits above as chips; the last one sends. Over
// `session-many-questions`.

import { kit } from "../sessions/walk-kit";
import { button, inside, region, role, text, walk } from "../walk";

const { rail } = kit(false);
const ask = role("article", "Waiting on you");
const thread = region("Thread");

const questionDeck = walk("session-many-questions", [
  { press: rail("Sessions"), say: "The Jobs page" },
  { press: inside(region("Jobs", { exact: true }), button(/Plan the night/)), say: "A Job asking four questions" },
  { press: inside(region("Needs you"), button(/Open Should a destructive best answer/)), say: "The question is a row in the panel, and a press opens it" },
  { look: inside(ask, text("Should a destructive best answer be decided overnight?")), say: "One question in front, three behind" },
  { press: inside(ask, button(/Destructive/)), say: "Pressing the band folds the card to it, so the thread above can be read" },
  { press: inside(ask, button(/Destructive/)), say: "Pressing it again opens the card, nothing picked is lost" },
  { press: inside(ask, role("radio", "Hold for me")), say: "Picking an answer" },
  { look: inside(ask, text("Should the Morning review open when Sleep goes off?")), say: "Brings the next one forward" },
  { press: inside(ask, role("radio", "Only if non-empty")), say: "And the next" },
  { press: inside(ask, role("radio", "Left of Helm")), say: "And the next" },
  { look: inside(ask, role("group", "Answered")), say: "What is answered sits above; press one to change it" },
  { press: inside(ask, role("radio", "Approve all")), say: "The last one" },
  { press: inside(ask, button("Answer")), say: "Sends them all" },
  { look: inside(thread, text("Are the five proposed icons right?")), say: "What was chosen is a message of yours in the thread: each question in bold, your answer under it" },
  { look: inside(thread, text("Approve all")), say: "The answer sits under its question" },
]);

export { questionDeck };
