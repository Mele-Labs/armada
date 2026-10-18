// Who each entry on the merge line came from: the Session that holds its pull request,
// and nothing at all where nobody here does. The Session's chip takes a hover
// and a press as it does everywhere, and its card opens the Session. Told twice: wide, and below
// the breakpoint.

import { NARROW } from "../sessions/walk-kit";
import { button, inside, region, role, viaPalette, walk } from "../walk";
import type { Step } from "../walk";

function steps(): Step[] {
  const line = region("Merge line");
  const row = (branch: string) => inside(line, role("listitem", new RegExp(`^${branch}`)));
  const session = role("dialog", "Owned by Pin the store clock");
  return [
    ...viaPalette("Merge line", "The merge line, three branches waiting"),
    { look: inside(row("fix/pin-store-clock"), button("Pull request #1861")), say: "A Job holds this one: its title is on the row" },
    { look: row("docs/typo-in-the-readme"), say: "Nobody here holds this one: nothing is drawn" },
    { hover: inside(row("fix/pin-store-clock"), button("Pull request #1861")), say: "Hover draws the Job's card" },
    { press: inside(row("fix/pin-store-clock"), button("Pull request #1861")), say: "A press keeps the card up" },
    { look: session, say: "Title, state, slot, pull request and last turn" },
    { press: inside(session, button("Open Job")), say: "It opens the Job" },
  ];
}

const wide = walk("merge-line-owners", steps());
const narrow = walk("merge-line-owners", steps(), NARROW);

export { wide as "merge-line-owners", narrow as "merge-line-owners-narrow" };
