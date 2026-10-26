// The Field open large over the stage, and the plan along its foot. `e` opens it, Esc puts it back. Over the `field-expand-plan`
// scenario.

import { deckRow, inside, region, role, walk } from "../walk";

const field = region("Field");
const large = role("group", "Expanded Field");

export const fieldExpandPlan = walk("field-expand-plan", [
  { press: deckRow("Rotate the audit log"), say: "The Job is open. The Field is in the right column, below the Blueprint" },
  { key: "e", on: field, say: "E opens the Field large over the stage. The thread and composer stay under it" },
  { look: inside(large, role("group", "Plan")), say: "The plan along the foot of the Field, a segment for each task. Solid is done with edits seen, outlined is done with none seen, amber is working, faint is open. Each plot a group names wears the same mark" },
  { hover: inside(large, role("listitem", /^List rotated files in System/)), say: "A segment names its task and its state" },
  { look: large, say: "The Field large, with the plan along its foot and each group named under its stretch" },
  { key: "Escape", on: large, say: "Esc is Back, to the thread as it was" },
]);
