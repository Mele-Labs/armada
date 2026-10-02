// A kept brief read in Pulse's log panel (owner, 2 Oct): a Judge's brief and a
// gaming check's, each opened by pressing its row, numbered as the file is and
// ending on the question the call answered. The row's Open still leaves the app.
// A brief opens wrapped, being prose, and the panel's toggle flips it.

import { button, dialog, inside, tab, text, walk } from "../walk";

export const pulseBriefPanel = walk("job/escalatedEvidenceSuspect", [
  { press: tab("Pulse"), say: "The Job a Judge refused, and every brief it kept" },
  { press: button("Judge brief, regression_verify · c2"), say: "The refused criterion's brief opens, its lines numbered in a gutter of their own" },
  { look: inside(dialog("Judge brief"), text("still assert that selectVisibleColumns memoises")), say: "It ends on the question the Judge answered" },
  { press: inside(dialog("Judge brief"), button("Close")), say: "Back to the list" },
  { press: button("Judge brief, regression_verify · gaming check"), say: "The gaming check's brief opens the same way" },
  { look: inside(dialog("Judge brief"), text("asserts less than it did")), say: "Asked whether an assertion was weakened" },
  { press: inside(dialog("Judge brief"), button("Wrap lines")), say: "A brief opens wrapped; the wrap toggle, named by its tooltip, unwraps it" },
  { look: inside(dialog("Judge brief"), button("Wrap lines")), say: "Unwrapped, a long line scrolls sideways under its number" },
]);
