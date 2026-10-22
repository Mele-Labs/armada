// A Session's pull request panel shows the line the pull request is in: the pull requests in the
// merge queue in order, each opening its own address, this one marked and its place the leading
// figure, on a plate beside the panel. #1849, in no queue, draws nothing. Over the `session-merge-line` scenario. Told twice:
// wide, and below the breakpoint.

import { kit, NARROW, toSessions } from "../sessions/walk-kit";
import { button, inside, region, role, text, walk } from "../walk";
import type { Step } from "../walk";

function steps(narrow: boolean): Step[] {
  const { sheet, close, row } = kit(narrow);
  const sessions = region("Jobs", { exact: true });
  const card = sheet("Pull request #1850");
  // The plate beside the panel; the Cockpit draws its own merge queue track too.
  const queue = inside(region("Merge line", { exact: true }), role("list", "Merge queue"));
  const ahead = inside(queue, role("listitem", /^Cap the log reader's buffer/));
  const mine = inside(queue, role("listitem", /^Record each migration once/));
  return [
    { press: inside(sessions, button("Store migration spike")), say: "A Job holding two pull requests" },
    ...row(/^#1849 /, "The first is in no queue"),
    { look: sheet("Pull request #1849"), say: "Nothing is drawn under the card" },
    { press: close(sheet("Pull request #1849")), say: "Back to the Job" },
    ...row(/^#1850 /, "The second is in the merge queue"),
    { look: queue, say: "The queue in order: two ahead, this one ringed, one behind" },
    { hover: inside(mine, role("img", "In the merge queue, waiting its turn")), say: "Its mark names where it stands" },
    { hover: inside(mine, text("3")), say: "The place figure is named" },
    { hover: ahead, say: "A row ahead" },
    { press: inside(ahead, role("link", /Cap the log reader's buffer/)), say: "Pressing it opens its own address" },
    { press: close(card), say: "Back to the Job" },
  ];
}

const wide = walk("session-merge-line", [toSessions, ...steps(false)]);
const narrow = walk("session-merge-line", [toSessions, ...steps(true)], NARROW);

export { wide as "session-pr-merge-line", narrow as "session-pr-merge-line-narrow" };
