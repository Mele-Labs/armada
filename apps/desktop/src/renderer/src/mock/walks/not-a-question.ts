// A sentence Fleet read out of a reply as an ask draws its own glyph and ends in Not a question; pressing it takes the row off. Over the
// `not-a-question` scenario.

import { button, deckRow, inside, region, role, text, walk } from "../walk";

const needs = region("Needs you");

const notAQuestion = walk("not-a-question", [
  { press: deckRow("Trim the Progress header"), say: "Its last reply ends in a plan's summary line" },
  { press: inside(needs, button(/^Needs you/)), say: "Two items: one Fleet read from the reply, one the agent asked" },
  { hover: inside(needs, role("img", "Read from the reply")), say: "A row read from a reply has its own glyph, not the Reply one" },
  { press: inside(needs, button("Not a question")), say: "Not a question takes the row off, and Fleet keeps the sentence as no ask" },
  { look: inside(needs, text("Should the header keep the approved mark, or drop it?")), say: "The question the agent asked stays, and opens on its own" },
]);

export { notAQuestion as "not-a-question" };
