// A plan says which pull request each group lands in: a placeholder chip until it opens, then the pull request, open and then merged. A Keep
// local Job's plan draws no chip, and its Artifacts keep what was closed from Needs you, with Reopen. Over `plan-pull-requests-and-closed`.

import { button, deckRow, inside, region, role, walk } from "../walk";

const artifacts = region("Artifacts", { exact: true });
const needs = region("Needs you");

const planPullRequestsAndClosed = walk("plan-pull-requests-and-closed", [
  { press: deckRow("Pair a phone over the local network"), say: "A plan waiting for approval" },
  { later: region("Lands in PR 1"), say: "Two groups land in PR 1, one in PR 2. Neither is opened" },
  { later: inside(region("Lands in #3101"), button(/^Pull request #3101/)), say: "PR 1 is open. PR 2 is not opened" },
  { look: inside(region("Lands in #3104"), role("img", "Merged")), say: "Both merged" },
  { press: deckRow("Tidy the bay list"), say: "A Keep local Job" },
  { look: inside(region("Job: Tidy the bay list"), role("article", "Plan to approve")), say: "Its plan has no chips" },
  { press: button("Artifacts", { exact: true }), say: "Artifacts" },
  { look: inside(artifacts, region("Closed", { exact: true })), say: "Closed: what was closed from Needs you. The question's call ended, so it has no Reopen" },
  { press: inside(artifacts, button(/^Reopen The bay list/)), say: "Reopen" },
  { press: inside(needs, button(/^Needs you/)), say: "The page is back in Needs you" },
]);

export { planPullRequestsAndClosed as "plan-pull-requests-and-closed" };
