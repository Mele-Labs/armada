// The timeline's Lanes take on the flight deck: a row for each Job and each Session, and pull requests as marks
// on the lane of the Job or Session that owns them, never a row of their own. Over `cockpit-timeline-first`.

import { button, inside, region, role, tab, walk } from "../walk";

const timeline = region("Timeline");

const cockpitBarOutTimelineFirst = walk("cockpit-timeline-first", [
  { press: inside(region("Flight controls"), button("Timeline")), say: "Timeline, from the flight controls" },
  { press: tab("Lanes"), say: "Lanes, a row for each subject" },
  { look: inside(timeline, role("list", "Moving")), say: "A row for each Job and each Session. No row is a pull request" },
  { hover: inside(timeline, button(/Queued, place 1/)), say: "A Job's lane carries its pull request: this mark is #1890 queued, the square shape marking a pull request" },
  { hover: inside(timeline, button(/Check store passed/)), say: "A Session's lane carries its own: #1896 on the branch the Session works on" },
]);

export { cockpitBarOutTimelineFirst as "cockpit-bar-out-timeline-first" };
