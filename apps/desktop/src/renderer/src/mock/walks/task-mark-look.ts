// A plan task Fleet marked itself, and one in progress that looks left behind. Both keep the task mark they had; hovering names the proof, or the reason and the
// age, and the stale one is dimmed. Over the `task-mark-look` scenario.

import { button, deckRow, inside, region, role, walk } from "../walk";

const blueprint = region("Blueprint", { exact: true });

export const taskMarkLook = walk("task-mark-look", [
  { press: deckRow("Rotate the audit log"), say: "The Job is open. The Blueprint reads as Now" },
  { press: inside(blueprint, button("Whole blueprint")), say: "One press opens the whole plan, a mark for every task" },
  { hover: inside(blueprint, role("img", /^Done\. Marked by Fleet/)), say: "Fleet marked this task done. The mark is the same check, and the tooltip names the proof" },
  { hover: inside(blueprint, role("img", /^Working\. Stale/)), say: "This task is in progress and the edits went elsewhere. The mark is dimmed, and the tooltip names why and how long" },
]);
