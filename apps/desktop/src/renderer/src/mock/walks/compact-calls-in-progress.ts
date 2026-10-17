// A take on where a call is answered: in the right column. The thread stays clean; calls stack at the top of the column, above Progress, each a
// compact card answered there, and the Field gives up room while one is open. Over the `compact-calls-in-progress` scenario. A design mock.

import { deckRow, inside, region, role, walk } from "../walk";

const compactCallsInProgress = walk("compact-calls-in-progress", [
  { press: deckRow("Retry the slot lease"), say: "The thread is clean. Calls stack at the top of the right column, above Progress, and the Field is smaller while one is open" },
  { type: "Four", into: inside(region("Waiting on you"), role("textbox", "Other")), say: "A press or a digit picks an answer. Typing is Other" },
  { key: "Enter", on: inside(region("Waiting on you"), role("textbox", "Other")), say: "↵ sends it, and the card is gone" },
]);

export { compactCallsInProgress };
