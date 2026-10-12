// A live Session forked from words in an earlier message. Selecting them raises a small bar over the
// selection; Fork starts a new Session from the conversation up to that message, opens it on the
// folded row linking back, and leaves the words in its box as a quote, unsent. The Session it came
// from goes on, and takes a message. Told twice, wide and below the breakpoint.

import { kit, NARROW, toSessions } from "../sessions/walk-kit";
import { button, inside, region, role, text, walk } from "../walk";
import type { Step } from "../walk";

const SAID = "When the second rolls over between the two reads, the values differ by one second.";

function steps(narrow: boolean): Step[] {
  const { message, thread, opened, row } = kit(narrow);
  const sessions = region("Sessions");
  const from = role("group", "Forked from Fix the flaky store test");
  return [
    { press: inside(sessions, button("Fix the flaky store test")), say: "A live Session opens" },
    { look: inside(thread, text("The test builds its expected timestamp")), say: "Its finding is a few messages up" },
    { select: SAID, in: inside(thread, text("The test builds its expected timestamp")), say: "Words in that message, selected" },
    { look: role("toolbar", "Selection"), say: "A bar sits just over the selection" },
    { hover: button("Fork from here"), say: "Its one act is named on hover" },
    { press: button("Fork from here"), say: "Fork starts a new Session from here" },
    { look: region("Session s-01FORK00"), say: "It opens at once" },
    { look: from, say: "Its thread opens on one quiet row, closed" },
    { press: role("button", "Forked from Fix the flaky store test", { exact: true }), say: "Pressed, it opens onto the old conversation" },
    { look: inside(from, text("When the second rolls over")), say: "Up to the message the words were in" },
    { look: message, say: "The words wait in its box as a quote, unsent" },
    ...opened([{ look: inside(role("region", "Forks"), text("Forked from Fix the flaky store test")), say: "Its ledger says where it came from" }]),
    ...row("Open Forked from Fix the flaky store test", "The row opens the Session it came from"),
    { look: region("Session s-01SESSIO"), say: "The old Session, untouched" },
    ...opened([{ look: inside(role("region", "Forks"), text("Forked to Fix the flaky store test")), say: "Its ledger now points forward" }]),
    { type: "Pin the clock in the store, and check sweep.rs too.\n", into: message, say: "And it takes a message" },
    { look: inside(thread, text("and check sweep.rs too")), say: "Sent, in the old conversation" },
  ];
}

const wide = walk("session-fork-selection", [toSessions, ...steps(false)]);
const narrow = walk("session-fork-selection", [toSessions, ...steps(true)], NARROW);

export { wide as "session-fork-selection", narrow as "session-fork-selection-narrow" };
