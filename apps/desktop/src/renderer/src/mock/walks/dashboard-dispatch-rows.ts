// The Dashboard's dispatch bar and its tiles. n and ⌘N bring the cursor to the bar from any tab and
// from another page; Tab chooses a Job or a Session. Running and Done are one grid of tiles: a
// Job's icon names what is active on it, a Session carries its cadence, a tile that needs the owner
// is lit, and the tile picked opens beside the grid. A Session's thread opens at its newest message
// and follows a new one. Done lists Jobs and Sessions only. Over `dashboard-dispatch-rows`.

import { region, role, tab, walk } from "../walk";

const request = role("textbox", "Request");
const tiles = role("listbox", "Tiles");
const thread = region("Thread");

const dashboardDispatchRows = walk("dashboard-dispatch-rows", [
  { press: tab("Running"), say: "Running, the cursor on the tab strip" },
  { key: "n", on: tab("Running"), say: "Press n" },
  { look: request, say: "The cursor is in the dispatch bar, a Job chosen, N and Tab drawn beside it" },
  { key: "Tab", on: request, say: "Press Tab" },
  { look: role("tab", "Session"), say: "A Session is chosen; the choice is remembered" },
  { type: "Check the migration order", into: request, say: "Words typed stay in the bar" },
  { key: "Enter", on: request, say: "Press Enter" },
  { look: role("textbox", "Message"), say: "The Session started and opened, holding what was typed" },
  { key: "Meta+n", on: role("textbox", "Message"), say: "Press ⌘N from inside the Session's message box" },
  { look: request, say: "The Dashboard, the cursor in the bar, Session still chosen" },
  { press: tab("Done"), say: "Done" },
  { look: role("listbox", "Items"), say: "Done is tiles too, Jobs and Sessions that are over; what landed in the merge line is not one" },
  { press: tab("Running"), say: "Running" },
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
