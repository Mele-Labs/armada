// The flight deck's Activity as a chart with no words, in three takes: a stream, a beeswarm and a radial. Each walk is one take, and the
// take follows the walk's name. Hovering shows the events near a point and pressing picks the newest of them. Over `activity-graph`.

import { button, inside, region, role, walk } from "../walk";

const activity = region("Activity");
const chart = inside(activity, role("group", "Last hour"));
// The newest thing drawn, whatever the clock reads: the fixture's ages are fixed when the page loads and the chart slides on from there.
const newest = inside(activity, button(/: /));
const time = inside(activity, role("slider", "Time"));

const activityStream = walk("activity-graph", [
  { look: chart, say: "Events over the last hour as layered bands, the newest at the right. Idle is at the bottom, then passed, working, failing and needs you on top. Reads, edits and commands are fainter" },
  { hover: time, say: "Hovering moves a thin line along the hour. The card lists the events within a minute and a half of it, newest first" },
  { press: time, say: "Pressing picks the newest of those events. The map rings the drone and dims the rest" },
  { key: "ArrowLeft", on: time, say: "The left and right arrows move the line a minute at a time" },
  { look: time, say: "A dot on the band marks the picked event" },
]);

const activityBeeswarm = walk("activity-graph", [
  { look: chart, say: "A dot for each event at its time, the newest at the right, packed up and down a centre line so none overlap. A dot is the colour of its tone. Reads, edits and commands are smaller and fainter" },
  { hover: newest, say: "Hovering a dot lists the events within a minute and a half of it, the hovered one first" },
  { press: newest, say: "Pressing a dot picks its event. The map rings the drone and dims the rest" },
  { key: "ArrowLeft", on: newest, say: "The left and right arrows move from dot to dot in time order" },
  { look: newest, say: "The picked event's dot is ringed" },
]);

const activityRadial = walk("activity-graph", [
  { look: chart, say: "A dial for the last hour. Now is at 12 o'clock and the hour runs counter-clockwise, so 15 minutes ago is at 9 o'clock. Each minute is a spoke that grows outward with its events" },
  { hover: newest, say: "Hovering a spoke lists that minute's events and its neighbours', newest first" },
  { press: newest, say: "Pressing a spoke picks its newest event. The map rings the drone and dims the rest" },
  { key: "ArrowLeft", on: newest, say: "The left and right arrows move from spoke to spoke" },
  { look: newest, say: "The picked event's spoke has a ring at its tip" },
]);

export { activityStream as "activity-stream", activityBeeswarm as "activity-beeswarm", activityRadial as "activity-radial" };
