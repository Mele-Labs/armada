// Sessions renamed from their headers, one of them untitled, and the Work list carrying the new name.
// Over the `session-names` scenario, which holds two terminal Sessions beside the usual ones:
// one whose first prompt opened with an agent message, and one holding fourteen pull requests, four of them merged.
// Told twice: wide, and below the breakpoint.

import { button, inside, region, role, walk } from "../walk";
import type { Step } from "../walk";
import { NARROW, toSessions } from "../sessions/walk-kit";

function steps(): Step[] {
  const sessions = region("Jobs", { exact: true });
  const frame = region("Job s5");
  return [
    { press: inside(sessions, button(/Review the ledger change/)), say: "Open the one from a terminal" },
    { look: frame, say: "The name is in the title bar, beside the Places" },
    { press: button("Review the ledger change, rename"), say: "A press on the name edits it where it stands" },
    { type: "Ledger review\n", into: role("textbox", "Job name"), say: "Typed, and Enter saves it" },
    { look: button("Ledger review, rename"), say: "A Job from a terminal is renamed the same way" },
    { look: inside(sessions, button("Ledger review")), say: "The list has the new name" },
    { look: inside(sessions, button("s7")), say: "One Job has no name yet, so the list shows its address" },
    { press: inside(sessions, button("s7")), say: "Opened" },
    { press: button("Rename"), say: "With no name to press, the title bar offers Rename as an icon" },
    { type: "Spike notes\n", into: role("textbox", "Job name"), say: "Typed, and Enter saves it" },
    { look: button("Spike notes, rename"), say: "An untitled Job is named from the title bar" },
  ];
}

const wide = walk("session-names", [toSessions, ...steps()]);
const narrow = walk("session-names", [toSessions, ...steps()], NARROW);

export { wide as "session-names", narrow as "session-names-narrow" };
