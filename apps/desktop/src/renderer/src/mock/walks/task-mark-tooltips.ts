// A task's mark on Plan's list names its state on hover, in the registry's word, for all six:
// a bare glyph gets a tooltip naming it. `TaskMark.stories.tsx` holds the claims.

import { inside, role, tab, walk } from "../walk";

const GROUPS = role("list", "Groups, in the order they run");
const mark = (state: string) => inside(GROUPS, role("img", state, { exact: true }));

export const taskMarkTooltips = walk("plan/every-task-state", [
  { press: tab("Plan"), say: "The plan, with a task in each of the six states" },
  { press: tab("List"), say: "As a list, every row led by its task's mark" },
  { hover: mark("Done"), say: "Done" },
  { hover: mark("Working"), say: "Working" },
  { hover: mark("Handed in"), say: "Handed in, and what it waits on" },
  { hover: mark("Failed"), say: "Failed" },
  { hover: mark("Open"), say: "Open" },
  { hover: mark("Dropped"), say: "Dropped" },
]);
