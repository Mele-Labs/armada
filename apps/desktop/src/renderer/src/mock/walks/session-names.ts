// The Sessions list with the name over its ledger, and Sessions renamed from their headers, one of them untitled.
// Over the `session-names` scenario, which holds two terminal Sessions beside the usual ones:
// one whose first prompt opened with an agent message, and one holding fourteen pull requests, four of them merged.
// Told twice: wide, and below the breakpoint.

import { button, inside, region, role, walk } from "../walk";
import type { Step } from "../walk";
import { NARROW, kit } from "../sessions/walk-kit";

function steps(narrow: boolean): Step[] {
  const { rail } = kit(narrow);
  const sessions = region("Sessions");
  const frame = region("Session s5");
  return [
    { look: inside(sessions, role("listitem", "Retire the sleeps")), say: "Fourteen pull requests, four of them merged. The row shows only the open ones, the newest that fit on one line, then a … holding the rest. The time is at the top right" },
    { look: inside(sessions, role("listitem", "Prune the stale branches")), say: "Amber edge: it is asking for a permission. Blue is working, green is idle with nothing asked, dim is not started. The checks icon sits inside the PR chip past a divider, coloured by how they stand, and the chip itself stays neutral" },
    { look: inside(sessions, role("listitem", "Review the ledger change")), say: "This one opened with an agent message, and shows the first line of what was said. Nothing is open on it, so it has no second line" },
    { press: rail("Sessions"), say: "The Sessions page" },
    { press: inside(sessions, button(/Review the ledger change/)), say: "Open the one from a terminal" },
    { look: frame, say: "The name is in the header" },
    { press: inside(frame, button("Review the ledger change, rename")), say: "A press on the name edits it where it stands" },
    { type: "Ledger review\n", into: inside(frame, role("textbox", "Session name")), say: "Typed, and Enter saves it" },
    { look: inside(frame, button("Ledger review, rename")), say: "A Session from a terminal is renamed the same way" },
    { press: rail("Sessions"), say: "Back to the list" },
    { look: inside(sessions, role("listitem", "Ledger review")), say: "The list has the new name" },
    { look: inside(sessions, role("listitem", "s7")), say: "One Session has no name yet, so the list shows its address" },
    { press: inside(inside(sessions, role("listitem", "s7")), button(/s7/)), say: "Opened" },
    { press: inside(region("Session s7"), button("Rename")), say: "With no name to press, the header offers Rename as an icon" },
    { type: "Spike notes\n", into: inside(region("Session s7"), role("textbox", "Session name")), say: "Typed, and Enter saves it" },
    { look: inside(region("Session s7"), button("Spike notes, rename")), say: "An untitled Session is named from its header" },
  ];
}

const wide = walk("session-names", steps(false));
const narrow = walk("session-names", steps(true), NARROW);

export { wide as "session-names", narrow as "session-names-narrow" };
