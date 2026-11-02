// Quick replies hidden from a Job and brought back, over the `livelier-thread` scenario: the bar's hide button, then Aa's switch.

import { deckRow, button, dialog, inside, role, walk } from "../walk";
import { toSessions } from "../sessions/walk-kit";

const quick = role("group", "Quick replies");

const off = walk("livelier-thread", [
  toSessions,
  {
    press: deckRow("Store clock fix"),
    say: "A Job with its quick bar",
  },
  {
    press: inside(quick, button("Hide quick replies")),
    say: "Hide the bar",
  },
  {
    press: role("menuitem", "Hide for this Job"),
    say: "Gone from this Job only",
  },
  {
    press: button("Thread look"),
    say: "Aa",
  },
  {
    press: inside(dialog("Thread look"), role("switch", "In this Job")),
    say: "Quick replies in this Job, back on",
  },
  {
    look: quick,
    say: "The bar is back",
  },
]);

export { off as "quick-replies-off" };
