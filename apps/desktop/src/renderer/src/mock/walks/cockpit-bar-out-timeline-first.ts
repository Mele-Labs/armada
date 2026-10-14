// The Cockpit after four notes: no dispatch bar under the title bar, Autopilot's bar at the very top where
// it stood, the timeline as the view it opens on, and pull requests as marks on the Job or Session that
// owns them instead of rows of their own. Over `cockpit-timeline-first`.

import { button, inside, region, role, tab, text, walk } from "../walk";

const timeline = region("Timeline");

const cockpitBarOutTimelineFirst = walk("cockpit-timeline-first", [
  { look: text("Pick a repository to start Autopilot"), say: "Autopilot's bar is the first thing on the Cockpit, above the filters and outside the glass. Nothing else stands between it and the title bar" },
  { look: timeline, say: "The Cockpit opens on the timeline" },
  { press: tab("Active"), say: "Active, the filter that reads everything live" },
  { look: tab("Lanes"), say: "Lanes, the timeline's own first take, is unchanged" },
  { look: inside(timeline, role("list", "Moving")), say: "A row for each Job and each Session. No row is a pull request" },
  { hover: inside(timeline, button(/Queued, place 1/)), say: "A Job's lane carries its pull request: this mark is #1890 queued, the square shape marking a pull request" },
  { hover: inside(timeline, button(/Check store passed/)), say: "A Session's lane carries its own: #1896 on the branch the Session works on" },
  { press: button("Grid"), say: "Grid is one press away, and the pick is kept" },
  { look: role("listbox", "Tiles"), say: "The tiles, as before" },
]);

export { cockpitBarOutTimelineFirst as "cockpit-bar-out-timeline-first" };
