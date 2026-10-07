// Who each entry on the merge line came from: the Session that holds its pull request, the Job
// whose branch it is, and nothing at all where nobody here does. The Session's chip takes a hover
// and a press as it does everywhere, and its card opens the Session. Told twice: wide, and below
// the breakpoint.

import { NARROW } from "../sessions/walk-kit";
import { button, inside, region, role, walk } from "../walk";
import type { Step } from "../walk";

function steps(): Step[] {
  const line = region("Merge line");
  const row = (branch: string) => inside(line, role("listitem", new RegExp(`^${branch}`)));
  const session = role("group", "Owned by Pin the store clock");
  return [
    { press: button("Merge line", { exact: true }), say: "The merge line, three branches waiting" },
    { look: inside(row("fix/pin-store-clock"), button("Pull request #1861")), say: "A Session holds this one: its title is on the row" },
    { look: inside(row("fix/61-order-store-migrations"), button("Order the store migrations")), say: "A Job's branch names the Job" },
    { look: row("docs/typo-in-the-readme"), say: "Nobody here holds this one: nothing is drawn" },
    { hover: inside(row("fix/pin-store-clock"), button("Pull request #1861")), say: "Hover draws the Session's card" },
    { press: inside(row("fix/pin-store-clock"), button("Pull request #1861")), say: "A press keeps the card up" },
    { look: session, say: "Title, state, slot, pull request and last turn" },
    { press: inside(session, button("Open Session")), say: "It opens the Session" },
  ];
}

const wide = walk("merge-line-owners", steps());
const narrow = walk("merge-line-owners", steps(), NARROW);

export { wide as "merge-line-owners", narrow as "merge-line-owners-narrow" };
