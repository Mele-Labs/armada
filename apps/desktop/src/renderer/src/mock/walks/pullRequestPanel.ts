// A Session's pull request as a panel, with plates floating off its leading edge. Told over three
// pull requests in the `session-pr-panel` scenario: third of seven in the merge queue, every check
// passed, and checks failed, which the Session can be sent back to fix. The first names its reviewers, opens each of its
// tabs and takes a comment; the second takes an approval. Only what Fleet serves is drawn. Told
// twice: wide, and below the breakpoint, where the plates move into the panel.

import { kit, NARROW, toSessions } from "../sessions/walk-kit";
import { button, inside, region, role, tab, text, walk } from "../walk";
import type { Step } from "../walk";

function steps(narrow: boolean): Step[] {
  const { close, row, sheet } = kit(narrow);
  const sessions = region("Jobs", { exact: true });
  const queued = sheet("Pull request #2209");
  const ready = sheet("Pull request #2210");
  const failed = sheet("Pull request #2213");
  const status = region("Status");
  const queue = role("list", "Merge queue");
  const box = role("form", "Comment box");
  return [
    { press: inside(sessions, button("Store migration spike")), say: "A Job holding three pull requests" },
    ...row("Open Pull request #2209", "The first is in the merge queue"),
    { look: inside(status, text("3rd in the merge queue")), say: "Its place leads the status plate" },
    { hover: inside(status, text("3")), say: "The figure is named" },
    { look: queue, say: "The queue in order, this pull request ringed" },
    { look: inside(queued, button("In merge queue")), say: "Already asked for, so the act is the fact" },
    { hover: role("button", "Open in browser"), say: "Open in browser is in the head" },
    { hover: inside(queued, text("nick")), say: "Who opened it, how long ago, and its size sit on the meta line" },
    { look: inside(status, text("No conflicts")), say: "The status plate says conflicts and the review" },
    { look: region("Reviewers"), say: "Who is a reviewer, one row each, the team marked as one" },
    { hover: inside(region("Reviewers"), role("img", "Approved")), say: "A verdict is a glyph, named on hover" },
    { hover: inside(region("Reviewers"), role("img", "Waiting")), say: "A reviewer who has not answered is waiting" },
    { hover: inside(region("Reviewers"), text("nina")), say: "A login is a link, and names the person on hover" },
    { look: inside(queued, tab("Conversation")), say: "The conversation is the first tab" },
    { look: inside(queued, text("Pull request links open the same way everywhere")), say: "The description leads, as Markdown" },
    { look: inside(queued, text("Every link goes through")), say: "Then what was said, in order" },
    { hover: inside(queued, role("img", "Changes requested")), say: "A review shows its verdict, named on hover" },
    { press: inside(queued, tab("Checks")), say: "Checks" },
    { look: inside(queued, role("list", "Checks")), say: "Each check: its state, its name, how long it ran, and its log" },
    { hover: inside(queued, text("2m 11s")), say: "The figure is named" },
    { press: inside(queued, tab("Files")), say: "Files" },
    { look: inside(role("dialog", "Files"), role("list", "Files")), say: "The files as a tree of folders: each file its status, its name, the lines added and removed" },
    { press: inside(queued, tab("Commits")), say: "Commits" },
    { look: inside(queued, role("list", "Commits")), say: "Each commit: its sha, its headline, its author" },
    { press: inside(queued, tab("Conversation")), say: "Back to the conversation" },
    { look: inside(box, button("Comment", { exact: true })), say: "A comment with no words cannot be sent" },
    { type: "Checked against the queue.", into: inside(queued, role("textbox", "Comment")), say: "A comment, typed in the box pinned under the panel" },
    { key: "Meta+Enter", on: inside(queued, role("textbox", "Comment")), say: "Press \u2318\u21b5" },
    { look: inside(queued, text("Checked against the queue.")), say: "It joins the conversation" },
    { press: close(queued), say: "Back to the Job" },
    ...row("Open Pull request #2210", "The second has passed every check"),
    { look: inside(status, text("Ready to merge")), say: "The status plate alone, no queue to show" },
    { look: inside(ready, button("Merge", { exact: true })), say: "Merge is offered" },
    { look: inside(status, text("Review required")), say: "No review has been given" },
    { press: inside(box, tab("Approve")), say: "Approve" },
    { press: inside(box, button("Approve", { exact: true })), say: "An approval needs no words" },
    { look: inside(ready, role("img", "Approved")), say: "It joins the conversation as a review" },
    { look: inside(status, text("Approved")), say: "The status plate follows" },
    { press: close(ready), say: "Back to the Job" },
    ...row("Open Pull request #2213", "The third failed its checks"),
    { look: region("Failing checks"), say: "A plate names the checks that failed" },
    { look: inside(status, text("Conflicts")), say: "The branch conflicts with main" },
    { press: inside(region("Failing checks"), button("Notify Job")), say: "The Job is told which failed" },
    { look: inside(region("Failing checks"), button("Notified")), say: "It is asked once for these checks" },
    { press: close(failed), say: "Back to the Job" },
  ];
}

const wide = walk("session-pr-panel", [toSessions, ...steps(false)]);
const narrow = walk("session-pr-panel", [toSessions, ...steps(true)], NARROW);

export { wide as pullRequestPanel, narrow as pullRequestPanelNarrow };
