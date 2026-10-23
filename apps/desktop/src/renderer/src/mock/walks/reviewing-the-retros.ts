// The guided review in Armada fixes, on the Logbook (`docs/concepts/retro.md`, *Reviewing*):
// Review beside the tabs reads the open items together and draws them one at a
// time, with the model's reason, a thread to ask about the card, and Skip and
// Back. The mock Fleet orders the items as listed and folds two with the same
// headline. The review is kept while the window is open: leaving the Logbook for Kit and
// coming back finds the same card and its thread, and Review again asks on purpose.
// `retro-page.test.tsx` holds the claims.

import { button, region, role, text, toSystemPage, walk } from "../walk";

const ASK = role("textbox", "Ask about this item");

export const reviewingTheRetros = walk("retro/lessons", [
  ...toSystemPage("Retros", "The Logbook, with Armada fixes under the entries"),
  { press: button("Review", { exact: true }), say: "Review, beside the tabs" },
  { look: text(/^1 of \d+$/), say: "One item at a time, and where it stands" },
  { look: text("Why this one"), say: "The model's line on why it is here" },
  { look: button("Skip", { exact: true }), say: "Skip keeps it open" },
  { type: "Which Sessions did this cost?", into: ASK, say: "A question about the card" },
  { press: button("Send", { exact: true }), say: "Sent to Fleet with the thread so far" },
  { look: text(/does not show more than its own words say/), say: "Fleet's answer, under the card" },
  { press: button("Skip", { exact: true }), say: "On to the next" },
  { look: text(/^2 of \d+$/), say: "The second card" },
  { press: button("Back", { exact: true }), say: "Back to the first" },
  { look: text("Which Sessions did this cost?"), say: "Its thread is still there" },
  { look: region("Review"), say: "The review, whole" },
  { press: button("Skip", { exact: true }), say: "On to the second card again" },
  ...toSystemPage("Kit", "Leave Armada fixes in the middle of the review"),
  ...toSystemPage("Retros", "And come back"),
  { look: text(/^2 of \d+$/), say: "The same card, with no Review pressed and no wait" },
  { look: text(/^Reviewed .* ago$|^Reviewed just now$/), say: "How old the review is" },
  { press: button("Back", { exact: true }), say: "Back to the first" },
  { look: text("Which Sessions did this cost?"), say: "Its thread came back too" },
  { press: button("Review again", { exact: true }), say: "Review again asks the model on purpose" },
  { look: text(/^1 of \d+$/), say: "From the first card, the old thread gone" },
  { press: button("Back to the list"), say: "And back to the list" },
]);
