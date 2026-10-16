// The flight deck's Activity as a heatmap with no words: a row for each Job or Session, a cell for each three minutes of the last hour,
// the newest at the right. Hover shows a cell's events; pressing it picks the newest. Over `activity-graph`.

import { button, inside, region, role, walk } from "../walk";

const activity = region("Activity");
// The last cell that holds anything, whatever the clock reads: the fixture's ages are fixed when the page loads and the cells slide on
// from there, so a name that waits on one event would not survive the minute.
const cell = inside(activity, button(/: /));

const activityGraph = walk("activity-graph", [
  { look: inside(activity, role("group", "Last hour")), say: "A row for each Job or Session, the most recently active on top. A cell for each three minutes of the last hour, the newest at the right. A faint tick under the right edge is now" },
  { look: cell, say: "A cell is bluer the more happened in it. Reads, edits and commands count for less than turns and steps. A cell that holds a question for him or a failure is amber or red" },
  { hover: cell, say: "Hovering a cell lists that Job or Session's events in it, newest first, in a card beside the panel: its name, how long ago, and what it did" },
  { press: cell, say: "Pressing a cell picks its newest event, as pressing a row did. The map rings the drone and dims the rest" },
  { look: cell, say: "The cell holding the picked event is ringed" },
]);

export { activityGraph as "activity-graph" };
