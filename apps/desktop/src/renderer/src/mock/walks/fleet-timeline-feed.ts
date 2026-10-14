// The Cockpit's timeline, first take: a feed. One row for each thing Fleet did, newest first, each row naming
// the Job or Session it happened to. The Cockpit opens on it; the grid is its other face. Over `fleet-timeline`.

import { button, region, tab, walk } from "../walk";

const timeline = region("Timeline");

export const fleetTimelineFeed = walk("fleet-timeline", [
  { press: tab("Active"), say: "Active, the filter that reads everything live" },
  { press: tab("Feed"), say: "Feed, the first of three takes: one row for each event, where Lanes is the one he opens on" },
  { look: timeline, say: "The feed, newest first. A mark on a spine for each event, the subject named on the row: a Job or a Session. A pull request's events sit on its owner's rows, the number leading the detail" },
  { look: button(/Job: Cache the manifest read/), say: "A row whose subject needs him carries it in its edge: this Job asked for a command and has not been answered" },
  { look: button(/Job: Install the PWA shell offline/), say: "Quiet for longer than ten minutes while working reads the same way, in orange" },
  { look: button(/Job: Debounce/), say: "A Check red on a Job, in red" },
  { press: tab("Your move"), say: "Your move keeps the subjects that need him, have failed or have gone quiet" },
  { look: timeline, say: "Only their events" },
  { press: tab("Done"), say: "Done keeps what is over" },
  { look: timeline, say: "Landed, failed, merged and ended" },
]);
