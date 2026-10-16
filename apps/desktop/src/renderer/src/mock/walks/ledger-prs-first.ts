// The ledger draws Waiting on you, then Pull requests, then Branches. Over the `ledger-prs-first`
// scenario. Told twice: wide, and below the breakpoint, where the ledger folds into a sheet.

import { NARROW, kit, toSessions } from "../sessions/walk-kit";
import { button, inside, region, role, walk } from "../walk";
import type { Step } from "../walk";

function steps(narrow: boolean): Step[] {
  const { ledger } = kit(narrow);
  const sessions = region("Sessions");
  return [
    { press: inside(sessions, button("Lock the store migrations")), say: "A Session with an item waiting, a pull request and three branches" },
    ...(narrow ? [{ press: button("Attachments"), say: "The ledger, opened" } as Step] : []),
    { look: inside(ledger, role("heading", "Waiting on you")), say: "Waiting on you first" },
    { look: inside(ledger, role("heading", "Pull requests")), say: "Pull requests directly under it" },
    { look: inside(ledger, role("heading", "Branches")), say: "Branches after them" },
  ];
}

const wide = walk("ledger-prs-first", [toSessions, ...steps(false)]);
const narrow = walk("ledger-prs-first", [toSessions, ...steps(true)], NARROW);

export { wide as "ledger-prs-first", narrow as "ledger-prs-first-narrow" };
