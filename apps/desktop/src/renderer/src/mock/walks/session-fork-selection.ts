// A live Session forked from words in an earlier message. Selecting them raises a small bar over the
// selection; Fork starts a new Session from the conversation up to that message, opens it on the
// folded row linking back, and leaves the words in its box as a quote, unsent. The Session it came
// from goes on, and takes a message. Told twice, wide and below the breakpoint.

import { kit, NARROW, toSessions } from "../sessions/walk-kit";
import { deckRow, button, inside, region, role, text, walk } from "../walk";
import type { Step } from "../walk";

const SAID = "When the second rolls over between the two reads, the values differ by one second.";

function steps(narrow: boolean): Step[] {
  const { message, thread } = kit(narrow);
  const sessions = region("Jobs", { exact: true });
  const from = role("group", "Forked from Fix the flaky store test");
  return [
    { press: inside(sessions, button("Fix the flaky store test")), say: "A live Job opens" },
    { look: inside(thread, text("The test builds its expected timestamp")), say: "Its finding is a few messages up" },
    { select: SAID, in: inside(thread, text("The test builds its expected timestamp")), say: "Words in that message, selected" },
    { look: role("toolbar", "Selection"), say: "A bar sits just over the selection" },
    { hover: button("Fork from here"), say: "Its one act is named on hover" },
    { press: button("Fork from here"), say: "Fork starts a new Job from here" },
    { look: region("Job s-01FORK00"), say: "It opens at once" },
    { look: from, say: "Its thread opens on one quiet row, closed" },
    { press: role("button", "Forked from Fix the flaky store test", { exact: true }), say: "Pressed, it opens onto the old conversation" },
    { look: inside(from, text("When the second rolls over")), say: "Up to the message the words were in" },
    { look: message, say: "The words wait in its box as a quote, unsent" },
    { press: deckRow("Fix the flaky store test"), say: "The Job it came from opens from the list" },
    { look: region("Job s-01SESSIO"), say: "The old Job, untouched" },
    { type: "Pin the clock in the store, and check sweep.rs too.\n", into: message, say: "And it takes a message" },
    { look: inside(thread, text("and check sweep.rs too")), say: "Sent, in the old conversation" },
  ];
}

const wide = walk("session-fork-selection", [toSessions, ...steps(false)]);
const narrow = walk("session-fork-selection", [toSessions, ...steps(true)], NARROW);

export { wide as "session-fork-selection", narrow as "session-fork-selection-narrow" };
