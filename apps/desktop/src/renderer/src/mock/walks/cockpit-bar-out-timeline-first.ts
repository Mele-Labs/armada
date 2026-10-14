// The Cockpit after four notes: no dispatch bar under the title bar, the timeline as the view it opens
// on, pull requests as marks on the Job or Session that owns them instead of rows of their own, and
// Autopilot as a control in the header band that opens its bar at the very top for the picked repository,
// asking for one first where none is picked. Over `cockpit-timeline-first`.

import { button, inside, region, role, tab, walk } from "../walk";

const timeline = region("Timeline");
const autopilot = button("Autopilot", { exact: true });

const cockpitBarOutTimelineFirst = walk("cockpit-timeline-first", [
  { look: autopilot, say: "Autopilot is a control in the header band. Nothing stands between the title bar and the Cockpit" },
  { look: timeline, say: "The Cockpit opens on the timeline" },
  { press: tab("Active"), say: "Active, the filter that reads everything live" },
  { look: tab("Lanes"), say: "Lanes, the timeline's own first take, is unchanged" },
  { look: inside(timeline, role("list", "Moving")), say: "A row for each Job and each Session. No row is a pull request" },
  { hover: inside(timeline, button(/Queued, place 1/)), say: "A Job's lane carries its pull request: this mark is #1890 queued, the square shape marking a pull request" },
  { hover: inside(timeline, button(/Check store passed/)), say: "A Session's lane carries its own: #1896 on the branch the Session works on" },
  { press: autopilot, say: "No repository is picked, so Autopilot asks for one in the title row's repository menu" },
  { look: role("searchbox", "Find a repository"), say: "The same menu, filter field and all" },
  { press: role("menuitem", /armada/), say: "Pick armada" },
  { look: region("Autopilot"), say: "Its Autopilot panel opens at the very top of the Cockpit, above the filters" },
  { look: button("Autopilot", { exact: true }), say: "The control stays pressed while the panel is open" },
  { press: button("Autopilot", { exact: true }), say: "Pressing it again puts the panel away. Nothing running stops" },
  { press: button("Grid"), say: "Grid is one press away, and the pick is kept" },
  { look: role("listbox", "Tiles"), say: "The tiles, as before" },
]);

export { cockpitBarOutTimelineFirst as "cockpit-bar-out-timeline-first" };
