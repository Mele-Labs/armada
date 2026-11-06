// Answering the agent's message in place: hover it, press the ✅ response on the bar under it, and the quote goes back. Over the
// `real-thread-header-sizes` scenario.

import { button, deckRow, inside, region, role, text, walk as walkOver } from "../walk";
import { toSessions } from "../sessions/walk-kit";

const thread = region("Thread");
// The last message's bar: the engine takes the last of several, and it is the one the hover raises.
const acts = inside(thread, role("toolbar", "Message"));

const walk = walkOver("real-thread-header-sizes", [
  toSessions,
  {
    press: deckRow("Header Font Sizes"),
    say: "A Job's thread, its last message asking a question",
  },
  {
    hover: inside(thread, text(/Do you want the plan recorded/)),
    say: "Hover the message and the quick responses come up in the bar under it",
  },
  {
    press: inside(acts, button("Pick this")),
    say: "✅ sends the message back as a quote, and the bar keeps the ✅ pressed",
  },
]);

export { walk as "real-thread-pick-a-line" };
