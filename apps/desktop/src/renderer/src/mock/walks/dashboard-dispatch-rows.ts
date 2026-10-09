// The Dashboard's dispatch bar and its rows. n and ⌘N bring the cursor to the bar from any tab and
// from another page; Tab chooses a Job or a Session; Running rows hold their mark, title and
// preview in one shape; a Session's thread opens at its newest message and follows a new one; Done
// lists Jobs and Sessions only. Over `dashboard-dispatch-rows`.

import { region, role, tab, text, walk } from "../walk";

const request = role("textbox", "Request");
const lanes = role("listbox", "Lanes");
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
  { look: role("listbox", "Items"), say: "Done lists the Jobs and Sessions that are over; what landed in the merge line is not an item" },
  { press: tab("Running"), say: "Running" },
  { look: lanes, say: "Each row: its state mark first, the title truncating, the steps or the Session mark at the end, the preview under the title" },
  { hover: role("img", "Working"), say: "The mark names the state on hover" },
  { press: role("option", /^Migration notes/), say: "A Session picked" },
  { look: thread, say: "Its thread opens at the newest message" },
  { later: thread, say: "A message arrives" },
  { look: text("0043 also drops the index"), say: "The thread stays at the end and shows it" },
]);

export { dashboardDispatchRows as "dashboard-dispatch-rows" };
