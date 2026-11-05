// What reaches Needs you now. A plan recorded again on Deliver waits as Plan v6. A design plan Job on Present lists both its pages. A sketch left open behind a newer one
// is not pinned and does not wait. A Kit workflow is in the Workflow menu. Over `needs-you-missing-items`.

import { button, deckRow, inside, role, text, walk } from "../walk";

const needsYouMissingItems = walk("needs-you-missing-items", [
  {
    press: deckRow("Cap retries in the store client"),
    say: "Deliver, plan recorded again",
  },
  {
    look: role("article", "Plan to approve"),
    say: "Plan v6 is pinned in the thread with Approve",
  },
  {
    press: deckRow("Design the Operator handoff"),
    say: "Present, two pages",
  },
  {
    look: text("Operator design handoff"),
    say: "Now lists the page it fetched",
  },
  {
    look: text("The Operator takes over"),
    say: "Now lists the page its Draft step wrote",
  },
  {
    press: deckRow("Guard the pricing floor"),
    say: "Two sketches, the newer approved",
  },
  {
    look: inside(
      role("article", "Sketch: Pricing guardrails (alerts)", {
        exact: true,
      }),
      role("img", "Approved"),
    ),
    say: "The newer sketch is approved",
  },
  {
    look: button("Open sketch Pricing guardrails (DSCR only)", {
      exact: true,
    }),
    say: "The older one is open on the ledger and not pinned",
  },
  {
    press: button(/^Workflow Feature/),
    say: "The workflow name",
  },
  {
    look: role("menuitem", "doc_review", {
      exact: true,
    }),
    say: "doc_review from the Kit",
  },
]);

export { needsYouMissingItems as "needs-you-missing-items" };
