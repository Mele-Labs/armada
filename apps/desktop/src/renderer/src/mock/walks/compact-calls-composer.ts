// A take on where a call is answered: in the composer. One line over the input and the answers as numbered keys under it; nothing pinned over the
// thread. Over the `compact-calls-composer` scenario. A design mock: nothing here asserts a colour or a size.

import { button, deckRow, inside, region, role, walk } from "../walk";

const box = role("textbox", "Message");

const compactCallsComposer = walk("compact-calls-composer", [
  { press: deckRow("Retry the slot lease"), say: "The question is one line over the composer's input and its answers are numbered keys. A digit or a press picks one. Typing is Other, and ↵ sends" },
  { key: "ArrowRight", on: box, say: "With several calls the line shows the first. ← and → step through them, and there is no count" },
  { press: inside(role("group", "Waiting on you"), button("More")), say: "More opens the whole call over the thread" },
  { key: "Escape", on: region("Judge"), say: "Esc closes it" },
]);

export { compactCallsComposer };
