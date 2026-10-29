// A full story: the real Issues tab Job with a long Done, ten marks in progress and three groups ahead. The column opens on Now and scrolls as one, a group whose
// tasks are listed draws no ticks, a folded one draws a slim track, and a tooltip sits over everything at the width of its words. Over `job-story-full`.

import { button, deckRow, inside, region, text, walk } from "../walk";

const now = region("Now", { exact: true });

export const jobStoryFull = walk("job-story-full", [
  { press: deckRow("Issues tab beside Reviews"), say: "A full Job. It opens on Now, with Done above and Open below, and the column scrolls as one" },
  { look: inside(now, button("Mock the Issues tab and walk it")), say: "A group with its tasks listed under it draws no circles" },
  { hover: inside(now, text(/^\d+ h/)), say: "Every tooltip sits above everything else and is as wide as its words" },
  { look: region("Open", { exact: true }), say: "A folded group draws one slim bar, a segment for each task, however many there are" },
  { key: "Alt+1", on: now, say: "⌥1 goes up to Done. Each section's header stays in view while you scroll through it" },
  { look: region("Done", { exact: true }), say: "Done's groups are folded, so each shows its bar" },
]);
