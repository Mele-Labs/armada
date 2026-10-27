// A draft the agent put to him is a card in the thread, approved where it stands: Approve, Edit and Note, and once settled how it ended.
// Words selected in the draft offer Edit and Add note from a right press. Over the `draft-card` scenario.

import { button, deckRow, inside, role, text, walk } from "../walk";

const card = role("article", "Draft: Description for #412", { exact: true });
const WORDS = "A retry resumes from the last part the server confirmed, not from the start";
const earlier = role("article", "Draft: Description for #412, first try");

const draftCard = walk("draft-card", [
  { press: deckRow("Retry dropped uploads"), say: "A Job with a drafted description" },
  { look: inside(card, role("group", "Answers")), say: "The draft card. Approve, Edit and Note under it" },
  { press: inside(card, button("Text it replaces")), say: "Text it replaces shows the old description" },
  { look: inside(card, text("Tested by hand on a slow connection.")), say: "The old description is under the new one" },
  { select: WORDS, in: inside(card, text(WORDS)), say: "Words selected in the draft" },
  { context: inside(card, text(WORDS)), say: "A right press on them offers Edit and Add note" },
  { press: role("menuitem", "Add note"), say: "Add note opens the note with the words quoted" },
  { press: inside(card, button("Cancel")), say: "Cancel puts the note away" },
  { select: WORDS, in: inside(card, text(WORDS)), say: "The same words selected again" },
  { context: inside(card, text(WORDS)), say: "A right press on them" },
  { press: role("menuitem", "Edit"), say: "Edit opens the draft in a box with those words selected" },
  { type: ", again", into: inside(card, role("textbox", "Draft text")), say: "A word added at the end of the text" },
  { press: inside(card, button("Approve with fixes")), say: "Approve with fixes" },
  { look: inside(card, role("img", "Approved with your fixes")), say: "The card is settled as approved with your fixes" },
  { look: text("Updated #412 with your text."), say: "The agent's reply is below it" },
  { look: inside(earlier, role("img", "Sent back")), say: "The earlier draft is settled as sent back, with its note" },
]);

export { draftCard as "draft-card" };
