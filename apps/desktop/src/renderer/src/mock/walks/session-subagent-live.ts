// Pressing a subagent in a Job's Subagents layer shows what it is doing: its own thread, drawn as the
// Session's is, gaining rows while it runs and ending on its report. Over the `session-subagent-live`
// scenario. Told twice: wide, and below the breakpoint.

import { button, inside, region, text, walk } from "../walk";
import type { Step } from "../walk";
import { NARROW, kit, toSessions } from "../sessions/walk-kit";

function steps(narrow: boolean): Step[] {
  const { key, sheet, close } = kit(narrow);
  const layer = region("Subagents", { exact: true });
  const sessions = region("Jobs", { exact: true });
  const running = "Read the CI history of store_flaky";
  const done = "Find other tests that read the wall clock";
  const panel = sheet(`Subagent ${running}`);
  const finished = sheet(`Subagent ${done}`);
  return [
    { press: inside(sessions, button("Read the CI history")), say: "A Job with two subagents, one still running" },
    { press: key("Subagents"), say: "Subagents lists both, the running one first" },
    { press: inside(layer, button(running)), say: "The running subagent is a press" },
    { look: inside(panel, region("Thread")), say: "Its own thread, drawn as the Job's is" },
    { look: inside(panel, text("gh run list --workflow ci --limit 30")), say: "What it has called so far" },
    { look: inside(panel, text("gh run view 1060 --log-failed")), say: "The sheet reads it again while it runs, so the thread gains rows" },
    { look: inside(panel, text("store_flaky failed 4 of the last 30 runs, each when the test crossed a second boundary.")), say: "It ends on its report, the last thing in the thread" },
    { press: close(panel), say: "Back to the list" },
    { press: inside(layer, button(done)), say: "The finished one is in the same list" },
    { look: inside(finished, text("store_flaky failed 4 of the last 30 runs, each when the test crossed a second boundary.")), say: "Its whole thread at once, ending on its report" },
    { press: close(finished), say: "Back to the list" },
    { press: inside(layer, button("Back")), say: "Back to the thread" },
  ];
}

const wide = walk("session-subagent-live", [toSessions, ...steps(false)]);
const narrow = walk("session-subagent-live", [toSessions, ...steps(true)], NARROW);

export { wide as "session-subagent-live", narrow as "session-subagent-live-narrow" };
