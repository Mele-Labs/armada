// The issues a Session works on, and their pull requests. A plan group that works an issue carries it on a meta line under its title in Now, and a press
// on the line or the group's title opens the issue in the side panel. The pull requests stand in Completed's list. An issue no group names stands in Completed with the Job's other issues.
// A press opens an issue or a pull request in the side panel, and Esc closes it. Over the `progress-issues` scenario.

import { button, deckRow, inside, region, role, walk } from "../walk";

const now = region("Now", { exact: true });
const landing = region("Completed", { exact: true });

const progressIssues = walk("progress-issues", [
  { press: deckRow("Fix Helm after a repository switch"), say: "Two issues, each worked by a group of the plan, and a pull request that closes neither. The pull requests are out, so Completed holds them" },
  { look: inside(landing, role("list", "Pull requests")), say: "Both pull requests, the one that closes no issue among them" },
  { press: inside(landing, button(/^Pull request #2256/)), say: "A pull request opens in the side panel" },
  { key: "Escape", on: role("dialog", "Pull request #2256"), say: "Esc closes it" },
  { press: inside(now, button("Whole plan")), say: "One press opens the whole plan" },
  { look: inside(now, button(/^Issue #2212/)), say: "A group that works an issue carries it on a line under its title" },
  { press: inside(now, button(/^Issue #2212/)), say: "An issue opens in the side panel" },
  { key: "Escape", on: role("dialog", "Issue #2212"), say: "Esc closes it" },
  { press: deckRow("Speed up the Cockpit list"), say: "No issue: its two pull requests alone" },
  { look: inside(landing, role("list", "Pull requests")), say: "Both stand in Completed" },
  { press: deckRow("Keep a quiet Session after reload"), say: "One issue and no plan, so no group works it" },
  { press: inside(landing, button(/^Issue #2214/)), say: "An issue no group names stands in Completed, and a press opens it in the side panel" },
  { key: "Escape", on: role("dialog", "Issue #2214"), say: "Esc closes it" },
  { press: deckRow("Work three Helm tickets"), say: "A plan of three groups, each naming an issue" },
  { press: inside(now, button("Whole plan")), say: "The whole plan" },
  { look: now, say: "Each group wears its issue on a line under its title" },
  { press: inside(region("Keep the draft through a resize"), button(/^Issue #2271/)), say: "A group's issue opens in the side panel" },
  { key: "Escape", on: role("dialog", "Issue #2271"), say: "Esc closes it" },
]);

export { progressIssues as "progress-issues" };
