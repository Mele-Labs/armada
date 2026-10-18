// The ledger draws Pull requests, then Branches, and what waits on the person stays in the thread. Over the
// `ledger-prs-first` scenario. Told twice: wide, and below the breakpoint.

import { NARROW, kit, toSessions } from "../sessions/walk-kit";
import { button, inside, region, role, walk } from "../walk";
import type { Step } from "../walk";

function steps(narrow: boolean): Step[] {
  const { ledger } = kit(narrow);
  const sessions = region("Jobs", { exact: true });
  return [
    { press: inside(sessions, button("Lock the store migrations")), say: "A Job with an item waiting, a pull request and three branches" },
    { look: region("Needs you"), say: "The item waiting on you is in the thread, over the done callout" },
    { look: inside(ledger, role("heading", "Pull requests")), say: "Pull requests first" },
    { look: inside(ledger, role("heading", "Branches")), say: "Branches after them" },
  ];
}

const wide = walk("ledger-prs-first", [toSessions, ...steps(false)]);
const narrow = walk("ledger-prs-first", [toSessions, ...steps(true)], NARROW);

export { wide as "ledger-prs-first", narrow as "ledger-prs-first-narrow" };
