// The diff lens on a Job's Field: the file a drone is writing lifts as a slab with its minimap, and a card at the top right types out
// its newest hunk. The card follows the drone to its next file and the first becomes a tab; three subagents writing at once lift
// smaller files with their counts; picking a drone in the Drones menu pins the card and Newest writer lets go. Over the `job-field-lens`
// scenario.

import { button, deckRow, inside, region, role, walk } from "../walk";

const field = region("Field", { exact: true });
const strip = inside(field, role("group", "Drones"));
// Found anywhere: with the pointer or focus in the panel, the card is drawn over the app, outside it.
const lens = (file: string) => role("group", `Lens: ${file}`, { exact: true });
const writing = inside(field, role("list", "Files being written"));
// The Drones menu's trigger, the one button over the map. Portalled, its menu is drawn on the body.
const drones = inside(strip, role("button"));
const item = (name: string | RegExp) => inside(role("menu"), role("menuitem", name));

export const jobFieldLens = walk("job-field-lens", [
  { press: deckRow("Show the change on the Field"), say: "The Job is open. Its drone is writing layout.ts" },
  { look: lens("layout.ts"), say: "layout.ts lifts off the floor as a slab: a minimap of the file, green where lines were added and red where they were removed, and its place on the floor dashed. The card at the top right names it, counts the lines, and shows the newest hunk" },
  { look: lens("layout.ts"), say: "While the drone writes, Fleet is asked again every two seconds. The lines new since the last answer type out, and the counts grow" },
  { later: lens("layout.ts"), say: "Now the drone moves on to FieldPanel.tsx" },
  { look: lens("FieldPanel.tsx"), say: "layout.ts settles back to the floor and FieldPanel.tsx lifts. The card follows it, and layout.ts is a faded tab behind it" },
  { later: lens("FieldPanel.tsx"), say: "Now three subagents start writing at once" },
  { look: writing, say: "Each file a subagent writes lifts too. The card follows the one that started last; the others are smaller, each with its count" },
  { look: lens("areas.rs"), say: "The card is on areas.rs, the newest writer's file" },
  { press: inside(strip, button(/^Keep the stamp in the store: Writing areas\.rs$/)), say: "The Drones menu over the map names the drone the card follows and what it is doing" },
  { press: item(/^Show the change on the Field/), say: "Picking the Job's own drone pins the card to it" },
  { look: lens("FieldPanel.tsx"), say: "The card is back on FieldPanel.tsx. The subagents' files stay lifted, smaller, with their counts" },
  { press: drones, say: "The menu again" },
  { press: item("Newest writer"), say: "Newest writer lets go" },
  { look: lens("areas.rs"), say: "The card follows the newest writer again" },
  { later: lens("areas.rs"), say: "Now the subagents finish" },
  { look: lens("FieldPanel.tsx"), say: "Their files settle back to the floor. The Job's drone still writes FieldPanel.tsx, and the card is on it" },
]);
