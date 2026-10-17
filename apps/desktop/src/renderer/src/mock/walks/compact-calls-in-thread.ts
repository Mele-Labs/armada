// Where a call is answered: in the Needs you panel on the composer, up over the thread when the question arrives, answered where it stands. Its
// trigger folds it and raises it again. Over the `compact-calls-in-thread` scenario. Approved 16 Oct 2026, before the card left the thread.

import { button, deckRow, role, walk } from "../walk";

const compactCallsInThread = walk("compact-calls-in-thread", [
  { press: deckRow("Retry the slot lease"), say: "The panel is up over the thread with the question, answered where it stands" },
  { look: role("article", "Waiting on you"), say: "The question, with its options" },
  { press: button("Needs you", { exact: true }), say: "Its trigger folds the panel, and the thread is clear" },
  { press: button("Needs you", { exact: true }), say: "And raises it again" },
]);

export { compactCallsInThread };
