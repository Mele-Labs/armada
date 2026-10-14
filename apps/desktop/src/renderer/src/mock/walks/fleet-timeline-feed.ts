// The Cockpit's timeline, first take: a feed. One row for each thing Fleet did, newest first, each row naming
// the Job, Session or pull request it happened to. The grid's other face, in place of the map. Over `fleet-timeline`.

import { button, region, tab, walk } from "../walk";

const timeline = region("Timeline");

export const fleetTimelineFeed = walk("fleet-timeline", [
  { press: tab("Active"), say: "Active, the filter that reads everything live" },
  { press: button("Timeline"), say: "The toggle beside the filter: grid or timeline, where it was grid or map" },
  { press: tab("Feed"), say: "Feed, the first of three takes: one row for each event, where Lanes is the one he opens on" },
  { look: timeline, say: "The feed, newest first. A mark on a spine for each event, the subject named on the row: a Job, a Session, a pull request" },
  { look: button(/Job: Cache the manifest read/), say: "A row whose subject needs him carries it in its edge: this Job asked for a command and has not been answered" },
  { look: button(/Job: Install the PWA shell offline/), say: "Quiet for longer than ten minutes while working reads the same way, in orange" },
  { look: button(/Pull request: #1894/), say: "A Check red on a pull request, in red" },
  { press: tab("Done"), say: "Done keeps what is over" },
  { look: timeline, say: "Landed, failed, merged and ended" },
]);
