// A plan recorded again after most of it was built: the groups nothing touched are one line each, the changed group is open with its
// marks, and U opens every closed group and closes them again. Over the `plan-revision-diff` scenario.

import { deckRow, inside, role, text, walk } from "../walk";

const card = role("article", "Plan to approve");

export const planRevisionDiff = walk("plan-revision-diff", [
  { press: deckRow("Warm the next bay"), say: "The Job is open, plan v5 waiting" },
  { look: card, say: "Two done groups are one line each. The changed group is open" },
  { look: inside(card, text("Say how long the warm took")), say: "Added, renamed with the old text struck, and the removed task struck" },
  { key: "u", on: card, say: "U opens every closed group" },
  { key: "u", on: card, say: "U closes them again" },
]);
