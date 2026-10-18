// The Cockpit's timeline, first take: a feed. One row for each thing Fleet did, newest first, each row naming
// the Session it happened to. The map and the timeline are one switch of the deck's stage. Over `fleet-timeline`.

import { button, inside, region, tab, walk } from "../walk";

const timeline = region("Timeline");
const controls = region("Flight controls");

export const fleetTimelineFeed = walk("fleet-timeline", [
  { press: inside(controls, button("Timeline")), say: "Timeline, from the flight controls" },
  { press: tab("Feed"), say: "Feed, the first of three takes: one row for each event, where Lanes is a row for each subject" },
  { look: timeline, say: "The feed, newest first. A mark on a spine for each event, the subject named on the row: a Job. A pull request's events sit on its owner's rows, the number leading the detail" },
  { look: button(/Job: Cache the manifest read/), say: "A row whose subject needs him carries it in its edge: this Job asked for a command and has not been answered" },
  { look: button(/Job: Install the PWA shell offline/), say: "Quiet for longer than ten minutes while working reads the same way, in orange" },
  { look: button(/Job: Debounce/), say: "A Check red on a Job, in red" },
  { press: inside(controls, button("Sessions only")), say: "Jobs only keeps the Jobs, as it narrows the column and the map" },
  { look: button(/Job: Flaky store test/), say: "Only their events" },
]);
