// The ledger of a Session: only the sections that hold a row, a small picture where it holds none,
// and the Artifacts section with a page, a file and a Doc. Over the `session-ledger`
// scenario. Told twice: wide, and below the breakpoint.

import { button, inside, region, role, walk } from "../walk";
import type { Step } from "../walk";
import { NARROW, kit } from "../sessions/walk-kit";

function steps(narrow: boolean): Step[] {
  const { ledger, opened, rail } = kit(narrow);
  const sessions = region("Sessions");
  return [
    { press: inside(sessions, button("Pin the store clock")), say: "A Session that holds a branch and a pull request" },
    ...opened([
      { look: inside(ledger, role("heading", "Branches")), say: "Branches, because it holds one" },
      { look: inside(ledger, role("heading", "Pull requests")), say: "Pull requests, because it holds one" },
    ]),
    { press: rail("Sessions"), say: "Back to the list" },
    { press: inside(sessions, button("Store clock spike")), say: "A Session that has not written anything yet" },
    ...opened([{ look: inside(ledger, role("img", "Nothing attached")), say: "No section, only a small picture" }]),
    ...(narrow
      ? []
      : [
          { press: button("Hide attachments"), say: "The button in the ledger panel's head hides it and the conversation takes the width" },
          { press: button("Show attachments"), say: "The conversation's header holds the button that shows it again" },
        ]),
    { press: rail("Sessions"), say: "Back to the list" },
    { press: inside(sessions, button("Write up the store clock")), say: "A Session that made three things" },
    ...opened([
      { look: inside(ledger, role("heading", "Artifacts")), say: "Artifacts" },
      { look: inside(ledger, role("img", "Published page")), say: "A page it published, which opens at its address" },
      { look: inside(ledger, role("img", "File written")), say: "A file it wrote outside the code, which opens on this machine" },
      { look: inside(ledger, role("img", "Doc")), say: "A Doc, which opens at its address" },
    ]),
  ];
}

const wide = walk("session-ledger", steps(false));
const narrow = walk("session-ledger", steps(true), NARROW);

export { wide as "session-ledger", narrow as "session-ledger-narrow" };
