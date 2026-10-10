// The title bar's one bar and the Dashboard's tiles. n and ⌘N open the bar from any filter and from
// another page; Tab chooses a Job or a Session. Active and Done are one grid of tiles: a
// Job's icon names what is active on it, a Session carries its cadence, a tile that needs the owner
// is lit, and the tile picked opens beside the grid. A Session's thread opens at its newest message
// and follows a new one. Done lists Jobs and Sessions only. Over `dashboard-dispatch-rows`.

import { button, inside, region, role, tab, walk } from "../walk";

const bar = role("combobox");
const tiles = role("listbox", "Tiles");
const thread = region("Thread");

const dashboardDispatchRows = walk("dashboard-dispatch-rows", [
  { press: tab("Active"), say: "Active, the cursor on the filter" },
  { key: "n", on: tab("Active"), say: "Press n" },
  { look: bar, say: "The title bar's one bar opens, ready for the work" },
  { key: "Tab", on: bar, say: "Press Tab: a Session is chosen, and the choice is remembered" },
  { type: "Check the migration order", into: bar, say: "Words typed" },
  { look: role("option", /^Start a Session with “Check the migration order”/), say: "They lead as a Session to start" },
  { key: "Enter", on: bar, say: "Press Enter" },
  { look: role("textbox", "Message"), say: "The Session started and opened, holding what was typed" },
  { key: "Meta+n", on: role("textbox", "Message"), say: "Press ⌘N from inside the Session's message box" },
  { look: bar, say: "The one bar again, Session still chosen" },
  { key: "Escape", on: bar, say: "Press Escape" },
  { press: inside(role("navigation", "Work"), button("Cockpit")), say: "Back to the Cockpit" },
  { press: tab("Done"), say: "Done" },
  { look: tiles, say: "Done is tiles too, Jobs and Sessions that are over; what landed in the merge line is not one" },
  { press: tab("Active"), say: "Active" },
  { look: tiles, say: "One grid: a state icon at the top left, the kind beside it, the title, then the steps or a Session's cadence and the age" },
  { hover: role("img", "Drone working"), say: "The icon names what is active on the Job" },
  { hover: role("img", "Waiting on you"), say: "A tile that needs the owner is lit" },
  { press: role("option", /^Cap the retry backoff/), say: "A Job picked" },
  { look: region("Now"), say: "Its pane says what is running now" },
  { key: "ArrowRight", on: tiles, say: "Press the right arrow" },
  { press: role("option", /^Migration notes/), say: "A Session picked" },
  { look: thread, say: "Its thread opens at the newest message" },
  { later: thread, say: "A message arrives" },
  { look: thread, say: "The thread stays at the end and shows it" },
]);

export { dashboardDispatchRows as "dashboard-dispatch-rows" };
