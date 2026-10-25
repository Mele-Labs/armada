// An agent that skips writing its answer is sent back for it: at a turn's end by Fleet's notice, and at its question, which
// waits on the owner only after the answer is written. Over the `unanswered-question` scenario.

import { kit, toSessions } from "../sessions/walk-kit";
import { button, deckRow, inside, role, text, walk } from "../walk";

const { thread } = kit(false);
const sentBack = inside(thread, button(/written him nothing but a status line/));
const needsYou = role("button", "Needs you");

const unansweredQuestion = walk("unanswered-question", [
  toSessions,
  { press: deckRow("Keep run memory out of the Logbook"), say: "A Job that twice skipped writing its answer" },
  { press: needsYou, say: "Fold the Needs you panel so the thread shows" },
  { look: sentBack, say: "Its turn ended with nothing written to you, so Fleet sent it back" },
  { look: inside(thread, text(/It's pushed on fix\/logbook-no-run-fold/)), say: "The answer it wrote then" },
  { look: inside(thread, text("Status: answering your question.")), say: "Next time, a status line and then straight to its question" },
  { look: inside(thread, text(/Commands were recorded so a folder/)), say: "Fleet sent the question back, so the answer comes first" },
  { press: needsYou, say: "Its question waits on you after the answer" },
]);

export { unansweredQuestion as "unanswered-question" };
