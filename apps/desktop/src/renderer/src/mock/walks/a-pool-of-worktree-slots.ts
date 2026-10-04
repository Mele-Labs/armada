// The worktree pool on Cleanup: one row per slot under a header naming each
// column, its state and build as marks in their hue, and leased, free and
// stranded rows tinted. A Job holding a slot is a link that opens it.

import { button, inside, role, text, walk } from "../walk";

const SLOT = (n: number) => role("row", new RegExp(`slot-${n}\\b`));

export const aPoolOfWorktreeSlots = walk("cleanup/slots", [
  { press: button("Cleanup", { exact: true }), say: "Cleanup, from the rail" },
  { look: text("Worktree slots"), say: "The pool, one row per slot" },
  { look: role("columnheader", "Behind main"), say: "Each column named: commits behind main" },
  { look: role("columnheader", "Held for"), say: "And how long it has been held" },
  { look: SLOT(1), say: "Leased: the row and its mark in the in-flight hue" },
  { hover: inside(SLOT(1), role("img", "Warm")), say: "Warm: its build is on disk" },
  { hover: inside(SLOT(1), role("button")), say: "The Job holding it: a Job mark and a link" },
  { look: inside(SLOT(2), text("zsh (pid 4120)")), say: "Held by a session, named by its process" },
  { look: inside(SLOT(2), role("cell", "7")), say: "Seven commits behind main" },
  { hover: inside(SLOT(3), role("img", "Free")), say: "Free: the row and its mark in the ready hue" },
  { hover: inside(SLOT(4), role("img", /Stranded/)), say: "Stranded: its holder is gone and it holds work" },
  { hover: inside(SLOT(5), role("img", "Cold")), say: "Free and cold: no build yet" },
  { hover: inside(SLOT(6), role("img", "Not made yet")), say: "Not made yet: neutral, the next lease makes it" },
  { press: inside(SLOT(1), role("button")), say: "The Job opens from its row" },
]);
