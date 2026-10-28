// Closing a Job puts the battlefield back at the whole field: the drone the cursor held no longer draws the camera in. Over the
// `battlefield-progress` scenario.

import { deckRow, inside, region, role, walk } from "../walk";

const map = role("listbox", "Map");
const stage = region(/^Job: /);
const dock = inside(map, role("option", "Rebuild the dock panel"));

const zoomsOut = walk("battlefield-progress", [
  { look: map, say: "The whole field, five drones in view" },
  { press: dock, say: "Pick a drone: the camera eases in on it, closer and lower" },
  { press: deckRow("Rebuild the dock panel"), say: "Open the Job. The map is put away under it" },
  { key: "Escape", on: stage, say: "Close the Job" },
  { look: map, say: "The map is back at the whole field, all five drones in view, and not drawn in on the Job just closed" },
]);

export { zoomsOut as "battlefield-zooms-out-on-close" };
