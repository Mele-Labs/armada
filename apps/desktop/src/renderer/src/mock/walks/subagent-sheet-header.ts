// A subagent's reading sheet states, in its head, the type of agent it is, the model it runs on and
// how full its context window is, as the Session's own head does for its window. Over the
// `session-subagent-live` scenario. Told twice: wide, and below the breakpoint.

import { button, inside, region, role, text, walk } from "../walk";
import type { Step } from "../walk";
import { NARROW, kit, toSessions } from "../sessions/walk-kit";

function steps(narrow: boolean): Step[] {
  const { ledger, opened, row, sheet, close } = kit(narrow);
  const sessions = region("Jobs", { exact: true });
  const running = "Subagent Read the CI history of store_flaky, running";
  const done = "Subagent Find other tests that read the wall clock, done";
  const panel = sheet("Subagent Read the CI history of store_flaky");
  const finished = sheet("Subagent Find other tests that read the wall clock");
  return [
    { press: inside(sessions, button("Read the CI history")), say: "A Job with two subagents" },
    ...row(`Open ${running}`, "A running subagent"),
    { look: inside(panel, text("general-purpose")), say: "The type of agent it is" },
    { look: inside(panel, text("Opus 5.5")), say: "The model it runs on" },
    { look: inside(panel, role("img", /^Context \d+%$/)), say: "How full its window is, drawn as the Job's is" },
    { look: inside(panel, region("Thread")), say: "Its own thread, as before" },
    { press: close(panel), say: "Back to the Job" },
    ...opened([{ press: inside(inside(ledger, region("Subagents")), role("radio", "All", { exact: true })), say: "The finished ones are behind All" }]),
    ...row(`Open ${done}`, "A finished subagent"),
    { look: inside(finished, text("rust-engineer")), say: "Another type" },
    { look: inside(finished, role("img", "Context 71%")), say: "Its window, as it stood at its last answer" },
    { press: close(finished), say: "Back to the Job" },
  ];
}

const wide = walk("session-subagent-live", [toSessions, ...steps(false)]);
const narrow = walk("session-subagent-live", [toSessions, ...steps(true)], NARROW);

export { wide as "subagent-sheet-header", narrow as "subagent-sheet-header-narrow" };
