// `progress-issues` where the tracker names issues by key: each ticket drawn by its key, with no `#`. The Session has no plan, so no group works a ticket: they
// stand in Deferred in Delivery, and the pull requests its rows name stand in Delivery's list. A press opens the ticket or a pull request in the side
// panel, and Esc closes it. Over the `progress-issues-jira` scenario.

import { button, deckRow, inside, region, role, walk } from "../walk";

const landing = region("Delivery", { exact: true });

const progressIssuesJira = walk("progress-issues-jira", [
  { press: deckRow("Work two checkout tickets"), say: "Two tickets, named by key. Delivery is the open panel: the pull requests are out" },
  { look: inside(landing, role("list", "Pull requests")), say: "The pull requests the tickets' rows name, the closed ticket's merged one among them" },
  { press: inside(landing, button("Deferred")), say: "No plan group works either ticket, so they stand in Deferred" },
  { press: inside(landing, button(/^Issue ABC-41/)), say: "A ticket opens in the side panel" },
  { key: "Escape", on: role("dialog", "Issue ABC-41"), say: "Esc closes it" },
  { press: inside(landing, button(/^Pull request #2303/)), say: "A pull request opens there too" },
  { key: "Escape", on: role("dialog", "Pull request #2303"), say: "Esc" },
]);

export { progressIssuesJira as "progress-issues-jira" };
