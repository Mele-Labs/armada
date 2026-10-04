// The worktree pool on Cleanup as a grid of bays, one per slot, styled by
// availability: held is a filled card under a band, free an open dashed
// outline, stranded hatched, not made a ghost. A Job holding a bay opens it.

import { button, inside, role, text, walk } from "../walk";

const BAY = (n: number) => role("listitem", `slot-${n}`, { exact: true });

export const aPoolOfWorktreeSlots = walk("cleanup/slots", [
  { press: button("Cleanup", { exact: true }), say: "Cleanup, from the rail" },
  { look: role("list", "Worktree slots"), say: "The pool, a bay per slot" },
  { look: BAY(1), say: "Held by a Job: a filled card under the leased band" },
  { hover: inside(BAY(1), role("button")), say: "The Job: its mark, and a link that opens it" },
  { hover: inside(BAY(1), role("img", "Warm")), say: "Warm: its build is on disk" },
  { hover: inside(BAY(2), text("7 behind")), say: "Seven commits behind main, named on hover" },
  { look: inside(BAY(2), text("zsh (pid 4120)")), say: "Held by a session, named by its process" },
  { look: BAY(3), say: "Free: an open bay, warm" },
  { look: BAY(4), say: "Stranded: hatched, and why" },
  { look: BAY(5), say: "Free and cold" },
  { look: BAY(6), say: "Not made yet: a ghost" },
  { press: inside(BAY(1), role("button")), say: "The Job opens from its bay" },
]);
