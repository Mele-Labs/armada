// The worktree pool on Cleanup: one row per slot, its state and build as marks
// named on hover, its branch, who holds it, for how long, and how far behind
// main it is. A Job holding a slot opens from its row.

import { button, inside, role, text, walk } from "../walk";

const SLOT = (n: number) => role("row", new RegExp(`slot-${n}\\b`));

export const aPoolOfWorktreeSlots = walk("cleanup/slots", [
  { press: button("Cleanup", { exact: true }), say: "Cleanup, from the rail" },
  { look: text("Worktree slots"), say: "The pool, one row per slot" },
  { hover: inside(SLOT(1), role("img", "Held")), say: "Held: a Job has it" },
  { hover: inside(SLOT(1), role("img", "Warm")), say: "Warm: its build is on disk" },
  { hover: inside(SLOT(1), text(/hours?$/)), say: "How long it has been held" },
  { look: inside(SLOT(2), text("claude (pid 4120)")), say: "Held by a session, named by its process" },
  { hover: inside(SLOT(2), text("7")), say: "Seven commits behind main" },
  { hover: inside(SLOT(3), role("img", "Free")), say: "Free, and warm" },
  { hover: inside(SLOT(4), role("img", /Stranded/)), say: "Stranded: its holder is gone and it holds work" },
  { hover: inside(SLOT(5), role("img", "Cold")), say: "Free and cold: no build yet" },
  { hover: inside(SLOT(6), role("img", "Not made yet")), say: "Not made yet: the next lease makes it" },
  { press: inside(SLOT(1), role("button")), say: "The Job holding slot-1 opens from its row" },
]);
