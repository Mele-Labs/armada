// The diff lens on a Job's Field: the file a drone is writing lifts as a slab with its minimap, and a card at the top right types out
// its newest hunk. The card follows the drone to its next file and the first becomes a tab; three subagents writing at once lift
// smaller files with their counts; the card goes on following the newest writer. Over the `job-field-lens`
// scenario.

import { deckRow, inside, region, role, walk } from "../walk";

const field = region("Field", { exact: true });
// Found anywhere: with the pointer or focus in the panel, the card is drawn over the app, outside it.
const lens = (file: string) => role("group", `Lens: ${file}`, { exact: true });
const writing = inside(field, role("list", "Files being written"));

export const jobFieldLens = walk("job-field-lens", [
  { press: deckRow("Show the change on the Field"), say: "The Job is open. Its drone is writing layout.ts" },
  { look: lens("layout.ts"), say: "layout.ts lifts off the floor as a slab: a minimap of the file, green where lines were added and red where they were removed, and its place on the floor dashed. The card at the top right names it, counts the lines, and shows the newest hunk" },
  { look: lens("layout.ts"), say: "While the drone writes, Fleet is asked again every two seconds. The lines new since the last answer type out, and the counts grow" },
  { later: lens("layout.ts"), say: "Now the drone moves on to FieldPanel.tsx" },
  { look: lens("FieldPanel.tsx"), say: "layout.ts settles back to the floor and FieldPanel.tsx lifts. The card follows it, and layout.ts is a faded tab behind it" },
  { later: lens("FieldPanel.tsx"), say: "Now three subagents start writing at once" },
  { look: writing, say: "Each file a subagent writes lifts too. The card follows the one that started last; the others are smaller, each with its count" },
  { look: lens("areas.rs"), say: "The card is on areas.rs, the newest writer's file" },
  { later: lens("areas.rs"), say: "Now the subagents finish" },
  { look: lens("FieldPanel.tsx"), say: "Their files settle back to the floor. The Job's drone still writes FieldPanel.tsx, and the card is on it" },
]);
