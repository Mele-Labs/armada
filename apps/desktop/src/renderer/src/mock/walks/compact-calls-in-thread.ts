// Where a call is answered: a pill over the composer, its sheet up over the thread when the question arrives, answered where it stands. The pill
// folds it and raises it again. Over the `compact-calls-in-thread` scenario. Approved 16 Oct 2026, before the card left the thread.

import { button, deckRow, role, walk } from "../walk";

const compactCallsInThread = walk("compact-calls-in-thread", [
  { press: deckRow("Retry the slot lease"), say: "The question's sheet is up over the thread, and is answered where it stands" },
  { look: role("article", "Waiting on you"), say: "The question, with its options" },
  { press: button("Attempts"), say: "Its pill folds the sheet, and the thread is clear" },
  { press: button("Attempts"), say: "And raises it again" },
]);

export { compactCallsInThread };
