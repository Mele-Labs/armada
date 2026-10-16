// The flight deck's Activity as a beeswarm with no words: a dot for each event over the last hour, the newest at the right. Hovering a dot
// shows the events near it and pressing it picks its event. Over `activity-beeswarm`.

import { button, inside, region, role, walk } from "../walk";

const activity = region("Activity");
const chart = inside(activity, role("group", "Last hour"));
// The newest dot, whatever the clock reads: the fixture's ages are fixed when the page loads and the chart slides on from there.
const newest = inside(activity, button(/: /));

const activityBeeswarm = walk("activity-beeswarm", [
  { look: chart, say: "A dot for each event at its time, the newest at the right, packed up and down a centre line so none overlap. A dot is the colour of its tone. Reads, edits and commands are smaller and fainter" },
  { hover: newest, say: "Hovering a dot lists the events within a minute and a half of it, the hovered one first" },
  { press: newest, say: "Pressing a dot picks its event. The map rings the drone and dims the rest" },
  { key: "ArrowLeft", on: newest, say: "The left and right arrows move from dot to dot in time order" },
  { look: newest, say: "The picked event's dot is ringed" },
]);

export { activityBeeswarm as "activity-beeswarm" };
