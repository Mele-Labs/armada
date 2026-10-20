// Progress lists the issues a Session works on, each with its pull requests under it, then the pull requests that close none of them. A press
// opens the issue or the pull request in the side panel, and Esc closes it. A plan group that names an issue is that issue's place, with its
// pull requests in the group. Over the `progress-issues` scenario; its presses are aimed at the Progress panel, where the issue or pull request stands.

import { button, deckRow, inside, region, role, walk } from "../walk";

const progress = region("Progress");
const issues = inside(progress, role("list", "Issues"));

const progressIssues = walk("progress-issues", [
  { press: deckRow("Fix Helm after a repository switch"), say: "Two issues and a pull request that closes neither" },
  { look: issues, say: "Each issue with its pull requests under it, then the pull request that closes none" },
  { press: inside(progress, button(/^Issue #2212/)), say: "An issue opens in the side panel" },
  { key: "Escape", on: role("dialog", "Issue #2212"), say: "Esc closes it" },
  { press: inside(progress, button(/^Pull request #2256/)), say: "Its pull request opens there too" },
  { key: "Escape", on: role("dialog", "Pull request #2256"), say: "Esc" },
  { press: deckRow("Speed up the Cockpit list"), say: "No issue: its two pull requests alone" },
  { press: deckRow("Work three Helm tickets"), say: "A plan of three groups, each naming an issue" },
  { look: progress, say: "Each group wears its issue before its title" },
  { press: inside(region("Keep the draft through a resize"), button(/^Issue #2271/)), say: "A group's issue opens in the side panel" },
  { key: "Escape", on: role("dialog", "Issue #2271"), say: "Esc closes it" },
]);

export { progressIssues as "progress-issues" };
