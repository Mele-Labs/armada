// A brief and a reviewer's comment read as the markdown they were written in.
// The owner's note of 1 Oct 2026, and his call that a review comment goes
// through the same Prose; `markdownFromTheProposer` is the other half.

import { card, dialog, inside, text, walk } from "../walk";

export const markdownFromAReviewer = walk("job/reviewAtDelivery", [
  { press: card("Brief"), say: "The brief, on the canvas's Brief" },
  {
    look: inside(dialog("Brief"), text("keep the public exports")),
    say: "The brief: whole store in bold, then a list with code in it",
  },
  {
    look: text("The test file has no case for an"),
    say: "A reviewer's comment: a paragraph, then a list with code in it",
  },
]);
