// Bugs agents reported in Armada itself: the Reported bugs band on the Issues tab, a report read whole, kept, started as a Job and
// dropped, all by key, and the mark on the Cockpit row of a Job that reported one. Over the `reported-bugs` scenario.

import { button, inside, place, region, role, walk } from "../walk";

const list = region("Issues", { exact: true });
const reader = region("Reported bug", { exact: true });

const reportedBugs = walk("reported-bugs", [
  { press: place("Issues"), say: "Issues, with All picked. Reported bugs stands over the repositories' issues" },
  { look: role("listitem", "Reported bugs", { exact: true }), say: "Each row names the report, the Job it came from and how long ago. The ring is a report not yet opened, the disk one you kept" },
  { press: inside(list, button(/^A watch stayed live/)), say: "Opening a report puts the ring away" },
  { look: role("region", "Errors", { exact: true }), say: "What went wrong, the numbered steps, the exact error lines and the log paths. From opens the Job" },
  { key: "K", on: reader, say: "K keeps it. Keep is gone and Start a Job takes its place" },
  { key: "s", on: reader, say: "s opens a drafted Job in the armada repository, holding the report as the brief" },
  { press: place("Issues"), say: "Issues again" },
  { key: "D", on: inside(list, button(/^waiting_for refused/)), say: "D on a row drops that report from the list" },
  { press: place("Cockpit"), say: "Cockpit" },
  { hover: button("Reported a bug", { exact: true }), say: "A Job that reported a bug wears this mark. A press opens its report on the Issues tab" },
]);

export { reportedBugs as "reported-bugs" };
