// The flight deck's Activity as a graph with no words: a column for each three minutes of the last hour, the newest at the right,
// each stacked by what the events were. Hover shows a column's events; pressing it picks the newest. Over `activity-graph`.

import { button, inside, region, role, walk } from "../walk";

const activity = region("Activity");

const activityGraph = walk("activity-graph", [
  { look: inside(activity, role("group", "Last hour")), say: "Activity is a graph and not a list. A column for each three minutes of the last hour, the newest at the right, with a faint tick under it for now" },
  { look: inside(activity, button(/^Pin the store clock: Edited 4 files/)), say: "A column is stacked by what happened in it: waiting and passed at the bottom, then working, and failing and needs you on top. Reads, edits and commands are the faint part of each" },
  { hover: inside(activity, button(/^Pin the store clock: Edited 4 files/)), say: "Hovering a column lists what happened in it, newest first, in a card beside the panel: the Job or Session, how long ago, and what it did. The glyph is in the colour of its bar" },
  { press: inside(activity, button(/^Pin the store clock: Edited 4 files/)), say: "Pressing the column picks its newest event, as pressing a row did: the map rings the drone and dims the rest" },
  { look: inside(activity, button(/^Pin the store clock: Edited 4 files/)), say: "The column holding the picked event is marked" },
]);

export { activityGraph as "activity-graph" };
