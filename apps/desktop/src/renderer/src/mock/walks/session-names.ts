// The Sessions list with the name over its ledger, and Sessions renamed from their headers, one of them untitled.
// Over the `session-names` scenario, which holds two terminal Sessions beside the usual ones:
// one whose first prompt opened with an agent message, and one holding six pull requests.
// Told twice: wide, and below the breakpoint.

import { button, inside, region, role, walk } from "../walk";
import type { Step } from "../walk";
import { NARROW, kit } from "../sessions/walk-kit";

function steps(narrow: boolean): Step[] {
  const { rail } = kit(narrow);
  const sessions = region("Sessions");
  const frame = region("Session s5");
  return [
    { look: inside(sessions, role("listitem", "Retire the sleeps")), say: "The name is its own line. Six pull requests wrap on the line under it, and the time is how long ago the last turn was" },
    { look: inside(sessions, role("listitem", "Review the ledger change")), say: "This one opened with an agent message, and shows the first line of what was said" },
    { press: rail("Sessions"), say: "The Sessions page" },
    { type: "j", into: sessions, say: "j goes to the next Session down the list and opens it" },
    { look: region("Session s2"), say: "The first one is open" },
    { type: "j", into: region("Session s2"), say: "j again" },
    { type: "j", into: region("Session s3"), say: "And once more" },
    { type: "k", into: frame, say: "k goes back up" },
    { look: region("Session s3"), say: "To the one above" },
    { type: "j", into: region("Session s3"), say: "Down to the Session this walk renames" },
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
