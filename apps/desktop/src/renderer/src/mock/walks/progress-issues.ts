// Progress lists the issues a Session works on, each with its pull requests under it, then the pull requests that close none of them. A press
// opens the issue or the pull request in the side panel, and Esc closes it. Over the `progress-issues` scenario. `?take=objectives`,
// `?take=strips` and `?take=berths` draw the block in each of the owner's three takes (`../issues-take.ts`).

import { button, deckRow, inside, region, role, walk } from "../walk";

const issues = inside(region("Progress"), role("list", "Issues"));

const progressIssues = walk("progress-issues", [
  { press: deckRow("Fix Helm after a repository switch"), say: "Two issues and a pull request that closes neither" },
  { look: issues, say: "Each issue with its pull requests under it, then the pull request that closes none" },
  { look: region("Progress"), say: "Its plan under the issues" },
  { press: inside(issues, button(/^Issue #2212/)), say: "An issue opens in the side panel" },
  { key: "Escape", on: role("dialog", "Issue #2212"), say: "Esc closes it" },
  { press: inside(issues, button(/^Pull request #2256/)), say: "Its pull request opens there too" },
  { key: "Escape", on: role("dialog", "Pull request #2256"), say: "Esc" },
  { press: deckRow("Speed up the Cockpit list"), say: "No issue: its two pull requests alone" },
]);

export { progressIssues as "progress-issues" };
