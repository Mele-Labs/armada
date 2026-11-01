// Answering one line of the agent's message in place: hover a list item, press the ✅ response, and the quote goes back. Over the
// `real-thread-header-sizes` scenario.

import { button, deckRow, inside, region, role, text, walk as walkOver } from "../walk";
import { toSessions } from "../sessions/walk-kit";

const thread = region("Thread");
const acts = inside(thread, role("group", /^Quick responses: C: 18/));

const walk = walkOver("real-thread-header-sizes", [
  toSessions,
  { press: deckRow("Header Font Sizes"), say: "A Job's thread, with the agent's options in a list" },
  { hover: inside(thread, text(/C: 18/)), say: "Hover an option and the quick responses float over its right end" },
  { press: inside(acts, button("Pick this")), say: "✅ sends the line back as a quote, and the line keeps a ✅" },
]);

export { walk as "real-thread-pick-a-line" };
