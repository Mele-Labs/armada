// Not a question as thumbs at the row's end: down takes the row off, up keeps it. Over the `not-a-question-thumbs` scenario.

import { button, deckRow, inside, region, role, text, walk } from "../walk";

const needs = region("Needs you");

const notAQuestionThumbs = walk("not-a-question-thumbs", [
  { press: deckRow("Trim the Progress header"), say: "Its last reply ends in a plan's summary line" },
  { press: inside(needs, button(/^Needs you/)), say: "Two items: one Fleet read from the reply, one the agent asked" },
  { hover: inside(needs, role("img", "Read from the reply")), say: "A row read from a reply has its own glyph, and two thumbs at its end" },
  { press: inside(needs, button("Not a question")), say: "Thumbs down takes the row off, and Fleet keeps the sentence as no ask" },
  { look: inside(needs, text("Should the header keep the approved mark, or drop it?")), say: "The question the agent asked stays, and opens on its own" },
]);

export { notAQuestionThumbs as "not-a-question-thumbs" };
