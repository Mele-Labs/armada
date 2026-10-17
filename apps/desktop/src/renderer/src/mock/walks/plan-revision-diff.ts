// A plan recorded again after most of it was built, drawn as what changed. The unchanged part opens and folds with U. Over the
// `plan-revision-diff` scenario; `&take=folded` plays the same steps on the second take.

import { deckRow, inside, role, text, walk } from "../walk";

const card = role("article", "Plan to approve");

export const planRevisionDiff = walk("plan-revision-diff", [
  { press: deckRow("Warm the next bay"), say: "The Job is open, plan v5 waiting" },
  { look: card, say: "The card leads with what changed, under its group" },
  { look: inside(card, text("Say how long the warm took")), say: "Added, renamed with the old text struck, and the removed task struck" },
  { key: "u", on: card, say: "U opens the unchanged part" },
  { key: "u", on: card, say: "U folds it again" },
]);
