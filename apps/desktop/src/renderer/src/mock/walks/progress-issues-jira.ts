// `progress-issues` where the tracker names issues by key: each ticket drawn by its key, with no `#`, and the pull requests its row names
// under it. A press opens the ticket or a pull request in the side panel, and Esc closes it. Over the `progress-issues-jira` scenario.

import { button, deckRow, inside, region, role, walk } from "../walk";

const progress = region("Progress");
const issues = inside(progress, role("list", "Issues"));

const progressIssuesJira = walk("progress-issues-jira", [
  { press: deckRow("Work two checkout tickets"), say: "Two tickets, named by key" },
  { look: issues, say: "Each ticket with the pull requests its row names, the closed one with its merged pull request" },
  { press: inside(progress, button(/^Issue ABC-41/)), say: "A ticket opens in the side panel" },
  { key: "Escape", on: role("dialog", "Issue ABC-41"), say: "Esc closes it" },
  { press: inside(progress, button(/^Pull request #2303/)), say: "Its pull request opens there too" },
  { key: "Escape", on: role("dialog", "Pull request #2303"), say: "Esc" },
]);

export { progressIssuesJira as "progress-issues-jira" };
