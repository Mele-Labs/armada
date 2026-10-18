// The ledger has no Worktree Slot section. Each branch row ends in the key chip of its slot, and a slot
// no branch is on keeps a row under Branches. Over the `ledger-slot-chips` scenario. Told twice: wide,
// and below the breakpoint.

import { NARROW, kit, toSessions } from "../sessions/walk-kit";
import { button, inside, region, role, walk } from "../walk";
import type { Step } from "../walk";

function steps(narrow: boolean): Step[] {
  const { ledger } = kit(narrow);
  const sessions = region("Jobs", { exact: true });
  return [
    { press: inside(sessions, button("Split the store migrations")), say: "A Job on five branches" },
    { look: inside(ledger, role("heading", "Branches")), say: "Branches, and no Worktree Slot section above it" },
    { look: inside(ledger, role("img", "Worktree slot 3")), say: "Each branch ends in the chip of its slot" },
    { look: inside(ledger, role("listitem", "Worktree slot 9")), say: "A slot with no branch keeps a row, last, with the key glyph" },
    { hover: inside(ledger, button("Open Branch store/backfill-orders")), say: "The card names the slot, and that the branch came with a Job" },
  ];
}

const wide = walk("ledger-slot-chips", [toSessions, ...steps(false)]);
const narrow = walk("ledger-slot-chips", [toSessions, ...steps(true)], NARROW);

export { wide as "ledger-slot-chips", narrow as "ledger-slot-chips-narrow" };
