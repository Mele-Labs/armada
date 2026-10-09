// From a worktree to the Session that holds it. Starts on Cleanup, where a slot
// a Session holds names it, and ends on Overview's Sessions list, searched by a
// pull request number. Told twice, wide and below the breakpoint.

import { kit, NARROW } from "../sessions/walk-kit";
import { button, inside, role, region, walk } from "../walk";
import type { Step } from "../walk";

function steps(narrow: boolean): Step[] {
  const { ledger, rail, sheet, close, row, opened } = kit(narrow);
  const owner = role("group", "Owned by Release notes script");
  const sessions = region("Sessions");
  return [
    { press: rail("Worktree Slots"), say: "Cleanup holds the worktree slots" },
    { look: role("button", "slot-5", { exact: true }), say: "Slot 5 is held by a Session, and says so" },
    { hover: button("Branch rel/notes-script"), say: "Its branch chip names its owner" },
    { press: button("Branch rel/notes-script"), say: "A press keeps the card up" },
    { look: owner, say: "Title, state, slot, pull requests, Jobs and last turn" },
    { press: inside(owner, button("Open Session")), say: "It opens the Session" },
    ...opened([{ look: inside(ledger, role("listitem", "Worktree slot 5")), say: "The slot is on its ledger" }]),
    ...row("Open Worktree slot 5", "The slot row opens the panel Cleanup's tiles open"),
    { look: sheet("slot-5"), say: "What the worktree holds, over the Session" },
    { press: close(sheet("slot-5")), say: "Back to the Session" },
    { press: rail("Sessions"), say: "Sessions lists every Session, under its own heading" },
    { look: inside(sessions, role("listitem", "Store migration spike")), say: "Each row shows its slot, its pull requests with their Checks, and its Jobs" },
    { type: "#1849", into: role("searchbox", "Search Sessions"), say: "A pull request number" },
    { look: inside(sessions, role("listitem", "Store migration spike")), say: "It finds the Session that owns it, and rings what matched" },
  ];
}

const wide = walk("sessions", steps(false));
const narrow = walk("sessions", steps(true), NARROW);

export { wide as "worktree-to-sessions", narrow as "worktree-to-sessions-narrow" };
