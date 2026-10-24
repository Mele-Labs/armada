// A sentence Fleet read out of a reply as an ask draws its own glyph and two thumbs at its end; a right press on the row offers the same two, and the line opened keeps the thumbs beside Back.
// Over the `not-a-question` scenario.

import { button, deckRow, inside, region, role, walk } from "../walk";

const needs = region("Needs you");
const summary = inside(needs, role("listitem", /What you get/));
const next = inside(needs, role("listitem", /^Next:/));

const notAQuestion = walk("not-a-question", [
  { press: deckRow("Trim the Progress header"), say: "Its last reply ends in two plan lines and a question" },
  { press: inside(needs, button(/^Needs you/)), say: "Three items: two Fleet read from the reply, one the agent asked" },
  { hover: inside(summary, role("img", "Read from the reply")), say: "A row read from a reply has its own glyph, and thumbs at its end" },
  { press: inside(summary, button(/^Open What you get/)), say: "Opened, the line keeps both thumbs beside Back" },
  { press: inside(needs, button("Not a question", { exact: true })), say: "Thumbs down from the opened line takes it off, returns to the list, and Fleet keeps the sentence as no ask" },
  { context: next, say: "A right press on the other one opens its menu, with the same two acts" },
  { press: role("menuitem", "A question", { exact: true }), say: "A question keeps the row, and Fleet keeps the sentence as an ask" },
  { look: next, say: "The row you confirmed stays without its thumbs" },
]);

export { notAQuestion as "not-a-question" };
