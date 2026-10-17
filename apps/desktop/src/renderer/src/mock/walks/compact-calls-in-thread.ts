// A take on where a call is answered: as the thread's last card, full height and answered in place. Nothing is pinned. Scrolled away from the end,
// a chip over the composer jumps back to it. Over the `compact-calls-in-thread` scenario. A design mock.

import { button, deckRow, role, walk } from "../walk";

const compactCallsInThread = walk("compact-calls-in-thread", [
  { press: deckRow("Retry the slot lease"), say: "Nothing is pinned. The call is the thread's last card, whole, and is answered where it stands" },
  { key: "Home", on: role("region", "Thread"), say: "Scrolled away from the end, a chip over the composer names the call" },
  { press: button("Question"), say: "The chip jumps back to the card and puts focus in it" },
]);

export { compactCallsInThread };
