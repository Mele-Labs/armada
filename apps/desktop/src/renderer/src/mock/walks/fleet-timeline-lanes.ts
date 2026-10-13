// The Cockpit's timeline, second take: lanes. A row for each Job, Session and pull request, its events as marks along
// the last hour, now at the right, the rows grouped by how they stand. Over `fleet-timeline`.

import { button, inside, region, role, tab, walk } from "../walk";

const timeline = region("Timeline");

export const fleetTimelineLanes = walk("fleet-timeline", [
  { press: tab("Active"), say: "Active, the filter that reads everything live" },
  { press: button("Timeline"), say: "Timeline, in place of the map" },
  { press: tab("Lanes"), say: "Lanes: a row for each subject instead of a row for each event" },
  { look: timeline, say: "Grouped by how each stands: needs him, failing, stuck, moving, waiting, finished. Time runs left to right, now at the right edge" },
  { look: inside(timeline, role("list", "Stuck")), say: "A solid line is a subject working, dashed is a subject that was not. Stuck ends in a dashed orange line that runs on to now" },
  { hover: inside(timeline, button(/Implement sent back/)), say: "A mark is one event, and names it with how long ago" },
  { hover: inside(timeline, button(/cargo test/)), say: "The ask nobody answered: the line to now is dashed in the ask colour" },
  { press: tab("Done"), say: "Done keeps what is over" },
  { look: timeline, say: "Landed work, drawn dimmer" },
]);
