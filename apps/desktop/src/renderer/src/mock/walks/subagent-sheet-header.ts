// A subagent's reading sheet states, in its head, the type of agent it is, the model it runs on and
// how full its context window is, as the Session's own head does for its window. Over the
// `session-subagent-live` scenario. Told twice: wide, and below the breakpoint.

import { button, inside, region, role, text, walk } from "../walk";
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
    { press: inside(sessions, button("Read the CI history")), say: "A Job with two subagents" },
    { press: key("Subagents"), say: "Subagents, the running one first" },
    { press: inside(layer, button(running)), say: "A running subagent" },
    { look: inside(panel, text("general-purpose")), say: "The type of agent it is" },
    { look: inside(panel, text("Opus 5.5")), say: "The model it runs on" },
    { look: inside(panel, role("img", /^Context \d+%$/)), say: "How full its window is, drawn as the Job's is" },
    { look: inside(panel, region("Thread")), say: "Its own thread, as before" },
    { press: close(panel), say: "Back to the list" },
    { press: inside(layer, button(done)), say: "A finished subagent, in the same list" },
    { look: inside(finished, text("rust-engineer")), say: "Another type" },
    { look: inside(finished, role("img", "Context 71%")), say: "Its window, as it stood at its last answer" },
    { press: close(finished), say: "Back to the list" },
    { press: inside(layer, button("Back")), say: "Back to the thread" },
  ];
}

const wide = walk("session-subagent-live", [toSessions, ...steps(false)]);
const narrow = walk("session-subagent-live", [toSessions, ...steps(true)], NARROW);

export { wide as "subagent-sheet-header", narrow as "subagent-sheet-header-narrow" };
