// A Session's pull request as a panel, with plates floating off its leading edge. Told over three
// pull requests in the `session-pr-panel` scenario: third of seven in the merge queue, every check
// passed, and checks failed, which the Session can be sent back to fix. Only what Fleet serves is
// drawn, so there are no tabs, comments, approvals or estimates. Told twice: wide, and below the
// breakpoint, where the plates move into the panel.

import { kit, NARROW, toSessions } from "../sessions/walk-kit";
import { button, inside, region, role, text, walk } from "../walk";
import type { Step } from "../walk";

function steps(narrow: boolean): Step[] {
  const { close, row, sheet } = kit(narrow);
  const sessions = region("Sessions");
  const queued = sheet("Pull request #2209");
  const ready = sheet("Pull request #2210");
  const failed = sheet("Pull request #2213");
  const status = region("Status");
  const queue = role("list", "Merge queue");
  return [
    { press: inside(sessions, button("Store migration spike")), say: "A Session holding three pull requests" },
    ...row("Open Pull request #2209", "The first is in the merge queue"),
    { look: inside(status, text("3rd in the merge queue")), say: "Its place leads the status plate" },
    { hover: inside(status, text("3")), say: "The figure is named" },
    { look: queue, say: "The queue in order, this pull request ringed" },
    { look: inside(queued, button("In merge queue")), say: "Already asked for, so the act is the fact" },
    { hover: role("button", "Open in browser"), say: "Open in browser is in the head" },
    { press: close(queued), say: "Back to the Session" },
    ...row("Open Pull request #2210", "The second has passed every check"),
    { look: inside(status, text("Ready to merge")), say: "The status plate alone, no queue to show" },
    { look: inside(ready, button("Merge", { exact: true })), say: "Merge is offered" },
    { press: close(ready), say: "Back to the Session" },
    ...row("Open Pull request #2213", "The third failed its checks"),
    { look: region("Failing checks"), say: "A plate names the checks that failed" },
    { press: inside(region("Failing checks"), button("Notify Session")), say: "The Session is told which failed" },
    { look: inside(region("Failing checks"), button("Notified")), say: "It is asked once for these checks" },
    { press: close(failed), say: "Back to the Session" },
  ];
}

const wide = walk("session-pr-panel", [toSessions, ...steps(false)]);
const narrow = walk("session-pr-panel", [toSessions, ...steps(true)], NARROW);

export { wide as pullRequestPanel, narrow as pullRequestPanelNarrow };
