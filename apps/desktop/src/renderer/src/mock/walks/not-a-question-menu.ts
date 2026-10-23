// Not a question from the row's own menu, opened with a right press or the ContextMenu key. Over the `not-a-question-menu` scenario.

import { button, deckRow, inside, region, role, text, walk } from "../walk";

const needs = region("Needs you");

const notAQuestionMenu = walk("not-a-question-menu", [
  { press: deckRow("Trim the Progress header"), say: "Its last reply ends in a plan's summary line" },
  { press: inside(needs, button(/^Needs you/)), say: "Two items: one Fleet read from the reply, one the agent asked" },
  { context: inside(needs, role("listitem", /What you get/)), say: "A right press on the row read from the reply opens its menu" },
  { press: role("menuitem", "Not a question"), say: "Not a question takes the row off, and Fleet keeps the sentence as no ask" },
  { look: inside(needs, text("Should the header keep the approved mark, or drop it?")), say: "The question the agent asked stays, and opens on its own" },
]);

export { notAQuestionMenu as "not-a-question-menu" };
