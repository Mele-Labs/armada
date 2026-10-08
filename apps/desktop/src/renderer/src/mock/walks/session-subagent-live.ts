// Pressing a subagent in a Session's ledger shows what it is doing: its own thread, drawn as the
// Session's is, gaining rows while it runs and ending on its report. Over the `session-subagent-live`
// scenario. Told twice: wide, and below the breakpoint.

import { button, inside, region, role, text, walk } from "../walk";
import type { Step } from "../walk";
import { NARROW, kit } from "../sessions/walk-kit";

function steps(narrow: boolean): Step[] {
  const { ledger, opened, row, sheet, close } = kit(narrow);
  const sessions = region("Sessions");
  const running = "Subagent Read the CI history of store_flaky, running";
  const done = "Subagent Find other tests that read the wall clock, done";
  const panel = sheet("Subagent Read the CI history of store_flaky");
  const finished = sheet("Subagent Find other tests that read the wall clock");
  return [
    { press: inside(sessions, button("Read the CI history")), say: "A Session with two subagents, one still running" },
    ...row(`Open ${running}`, "The running subagent is a press"),
    { look: inside(panel, region("Thread")), say: "Its own thread, drawn as the Session's is" },
    { look: inside(panel, text("gh run list --workflow ci --limit 30")), say: "What it has called so far" },
    { look: inside(panel, text("gh run view 1060 --log-failed")), say: "The sheet reads it again while it runs, so the thread gains rows" },
    { look: inside(panel, text("store_flaky failed 4 of the last 30 runs, each when the test crossed a second boundary.")), say: "It ends on its report, the last thing in the thread" },
    { press: close(panel), say: "Back to the Session" },
    ...opened([{ press: inside(inside(ledger, region("Subagents")), role("radio", "All", { exact: true })), say: "The finished ones are behind All" }]),
    ...row(`Open ${done}`, "A finished subagent"),
    { look: inside(finished, text("store_flaky failed 4 of the last 30 runs, each when the test crossed a second boundary.")), say: "Its whole thread at once, ending on its report" },
    { press: close(finished), say: "Back to the Session" },
  ];
}

const wide = walk("session-subagent-live", steps(false));
const narrow = walk("session-subagent-live", steps(true), NARROW);

export { wide as "session-subagent-live", narrow as "session-subagent-live-narrow" };
