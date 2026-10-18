// What a Session's waiting line offers the owner follows its act, read where the Session lists them: its
// Needs you. A slash command is the Session's own and is sent to it with Run; a shell command is a block
// with Run; a pull request opens; a plain line is a reply. The deck's call strip carries one
// act at a time, so the lines are told here. The standing answers of a question with options are on the call's
// strip, with nothing else to compare them to here. Over `session-errands`.

import { button, inside, region, role, walk } from "../walk";
import { toSessions } from "../sessions/walk-kit";

const needs = region("Needs you");
const slash = inside(needs, role("group", "/land 2109 2118 2119"));
const shell = inside(needs, role("group", "kill $(lsof -ti tcp:5788) && scripts/preview --restart --adopt"));

const sessionErrands = walk("session-errands", [
  toSessions,
  { press: inside(region("Jobs", { exact: true }), button(/Land the speed fixes/)), say: "A Job with five lines waiting on him" },
  { press: inside(needs, button(/^Needs you/)), say: "Needs you, raised from the composer" },
  { press: inside(needs, button(/^Open \/land 2109/)), say: "A slash command is a row, and a press opens it" },
  { look: inside(slash, button("Run")), say: "A slash command is for the Job itself: Run sends it as the reply" },
  { press: inside(needs, button("Back", { exact: true })), say: "Back to the list" },
  { press: inside(needs, button(/^Open Restart onto the preview/)), say: "A shell command is another row" },
  { look: shell, say: "A shell command is for him: the command itself, in the case it was written" },
  { look: inside(shell, button("Run")), say: "Run runs it where the Job works" },
  { press: inside(needs, button("Back", { exact: true })), say: "Back to the list" },
  { look: inside(needs, button("Open Approve #2118")), say: "A pull request to approve: the line opens it" },
  { look: inside(needs, role("img", "Opens the pull request")), say: "Marked as one that opens" },
  { look: inside(needs, button(/^Open Which branch should the migration rebase onto/)), say: "A plain line opens the Job's own box" },
  { look: inside(needs, role("img", "Reply")), say: "Marked as a reply, with no standing answers" },
]);

export { sessionErrands as "session-errands" };
