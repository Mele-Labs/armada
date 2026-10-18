// The ledger's bar of icons, which narrows it to one kind of thing, and its Search. Over the
// `ledger-filter-bar` scenario. Told twice: wide, and below the breakpoint.

import { NARROW, kit, toSessions } from "../sessions/walk-kit";
import { button, inside, region, role, walk } from "../walk";
import type { Step } from "../walk";

function steps(narrow: boolean): Step[] {
  const { ledger } = kit(narrow);
  const sessions = region("Jobs", { exact: true });
  const bar = inside(ledger, role("group", "Narrow the ledger"));
  return [
    { press: inside(sessions, button("Lock the store migrations")), say: "A done Job holding a pull request, branches in slots, a bare slot and artifacts" },
    { look: region("Needs you"), say: "The item waiting on you stays in the thread, over the done callout" },
    { look: bar, say: "A bar across the top of the ledger" },
    { press: inside(bar, button("Pull requests")), say: "Pull requests" },
    { look: inside(ledger, role("heading", "Pull requests")), say: "Only the pull request is left" },
    { press: inside(bar, button("Pull requests")), say: "Pull requests again" },
    { look: inside(ledger, role("heading", "Artifacts")), say: "Everything is back" },
    { press: inside(bar, button("Branches and slots")), say: "Branches and slots" },
    { look: inside(ledger, button("Open Worktree slot 9")), say: "The branches, and the slot no branch is on" },
    { press: inside(bar, button("Search")), say: "Search" },
    { type: "backfill", into: inside(bar, role("textbox", "Search the ledger")), say: "A fragment of a branch" },
    { look: inside(ledger, button("Open Branch store/backfill-orders")), say: "Branches, narrowed to the match" },
  ];
}

const wide = walk("ledger-filter-bar", [toSessions, ...steps(false)]);
const narrow = walk("ledger-filter-bar", [toSessions, ...steps(true)], NARROW);

export { wide as "ledger-filter-bar", narrow as "ledger-filter-bar-narrow" };
