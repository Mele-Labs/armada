// Where a call is answered: in the Needs you panel on the composer, up over the thread when the question arrives, answered where it stands. Its
// trigger folds it and raises it again. Over the `compact-calls-in-thread` scenario. Approved 16 Oct 2026, before the card left the thread.

import { button, deckRow, inside, region, role, walk } from "../walk";

const compactCallsInThread = walk("compact-calls-in-thread", [
  { press: deckRow("Retry the slot lease"), say: "The panel is up over the thread with the question, answered where it stands" },
  { press: inside(region("Needs you"), button(/Open How many attempts/)), say: "The question is a row in the panel, and a press opens it" },
  { look: role("article", "Waiting on you"), say: "The question, with its options" },
  { press: inside(region("Needs you"), button("Back", { exact: true })), say: "Back returns to the list" },
  { press: button("Needs you", { exact: true }), say: "Its trigger folds the panel, and the thread is clear" },
  { press: button("Needs you", { exact: true }), say: "And raises it again" },
]);

export { compactCallsInThread };
