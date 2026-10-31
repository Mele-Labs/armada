// Answering one line of the agent's message in place: hover a list item, press Pick, and the quote goes back. Over the
// `real-thread-header-sizes` scenario.

import { button, deckRow, inside, region, role, text, walk as walkOver } from "../walk";
import { toSessions } from "../sessions/walk-kit";

const thread = region("Thread");
const acts = inside(thread, role("group", /^Answer: C: 18px/));

const walk = walkOver("real-thread-header-sizes", [
  toSessions,
  { press: deckRow("Header Font Sizes"), say: "A Job's thread, with the agent's options in a list" },
  { hover: inside(thread, text(/C: 18px/)), say: "Hover an option and Pick, Yes and No appear at its end" },
  { press: inside(acts, button("Pick this")), say: "Pick sends the line back to the agent as a quote" },
  { look: inside(thread, role("img", "Picked")), say: "Your quote is in the thread and the line keeps its mark" },
]);

export { walk as "real-thread-pick-a-line" };
