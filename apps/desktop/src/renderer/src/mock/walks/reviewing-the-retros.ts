// The guided review on the Retros page (`docs/concepts/retro.md`, *Reviewing*):
// Review beside the tabs reads the open items together and draws them one at a
// time, with the model's reason, a thread to ask about the card, and Skip and
// Back. The mock Fleet orders the items as listed and folds two with the same
// headline. `retro.test.tsx` holds the claims.

import { button, region, role, text, walk } from "../walk";

const ASK = role("textbox", "Ask about this item");

export const reviewingTheRetros = walk("retro/lessons", [
  { press: button("Retros", { exact: true }), say: "Retros, under Work" },
  { press: button("Review", { exact: true }), say: "Review, beside the tabs" },
  { look: text(/^1 of \d+$/), say: "One item at a time, and where it stands" },
  { look: text("Why this one"), say: "The model's line on why it is here" },
  { look: button("Skip", { exact: true }), say: "Skip keeps it open" },
  { type: "Which Jobs did this cost?", into: ASK, say: "A question about the card" },
  { press: button("Send", { exact: true }), say: "Sent to Fleet with the thread so far" },
  { look: text(/does not show more than its own words say/), say: "Fleet's answer, under the card" },
  { press: button("Skip", { exact: true }), say: "On to the next" },
  { look: text(/^2 of \d+$/), say: "The second card" },
  { press: button("Back", { exact: true }), say: "Back to the first" },
  { look: text("Which Jobs did this cost?"), say: "Its thread is still there" },
  { look: region("Review"), say: "The review, whole" },
  { press: button("Back to the list"), say: "And back to the list" },
]);
