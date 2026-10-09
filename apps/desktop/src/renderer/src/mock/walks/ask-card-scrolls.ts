// Four questions at once: the card stops at most of the window and scrolls, so the last question
// and its answer are reachable and the thread above stays in sight. Over `session-many-questions`.

import { kit } from "../sessions/walk-kit";
import { button, inside, region, role, text, walk } from "../walk";

const { rail } = kit(false);
const ask = role("article", "Waiting on you");

const askCardScrolls = walk("session-many-questions", [
  { press: rail("Sessions"), say: "The Sessions page" },
  { press: inside(region("Sessions"), button(/Plan the night/)), say: "A Session asking four questions" },
  { look: inside(ask, text("Should a destructive best answer be decided overnight?")), say: "The first question" },
  { look: inside(ask, text("Are the five proposed icons right?")), say: "The card scrolls to the last" },
  { look: inside(ask, button("Answer")), say: "And to the answer" },
  { look: text("The walk is open. Four things to settle."), say: "The thread stays above it" },
]);

export { askCardScrolls };
