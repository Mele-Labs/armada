// Who held a stranded slot last, on its bay: the title as a link while it can be opened, a mark for
// the one telling Fleet gave, a mark for a holder that is gone and one for the Finding already read.

import { inside, role, viaPalette, walk } from "../walk";

const BAY = (n: number) => role("listitem", `slot-${n}`, { exact: true });
const IN_BAY = (n: number, kind: string, name: string | RegExp) => inside(BAY(n), role(kind, name));

export const aStrandedSlotsLastHolder = walk("cleanup/last-holder", [
  ...viaPalette("Worktree Slots", "Cleanup, from the rail"),
  { look: BAY(5), say: "Stranded, with the Job that left it under the branch" },
  { hover: IN_BAY(5, "button", "Pull Request Panel Design"), say: "Its title, a link that opens that Job" },
  { hover: IN_BAY(5, "img", /^Told /), say: "Fleet told it once to commit, push and release, and when" },
  { look: BAY(6), say: "A holder that is gone: its title, no link" },
  { hover: IN_BAY(6, "img", "Job closed"), say: "Closed" },
  { hover: IN_BAY(6, "img", "Finding ready"), say: "Nobody to tell, so Fleet ran the Scout, and its Finding is in the panel" },
  { look: BAY(7), say: "A Job that closed holds the same row" },
  { hover: IN_BAY(7, "img", "Job closed"), say: "Closed" },
  { look: BAY(4), say: "A holder Fleet never recorded draws nothing" },
  { press: IN_BAY(5, "button", "Pull Request Panel Design"), say: "The Job opens from its bay" },
]);
