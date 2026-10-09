// The Dashboard with nothing waiting on the owner: the cursor starts in the dispatch bar over the
// fleet board. Typing grows it into the composer. Over `dashboard-quick-dispatch`.

import { button, role, tab, text, walk } from "../walk";

const request = role("textbox", "Request");

const dashboardQuickDispatch = walk("dashboard-quick-dispatch", [
  { look: tab("Your move"), say: "Command Central, the tab the Dashboard opens on" },
  { look: request, say: "Nothing needs you, so the cursor starts in the dispatch bar" },
  { type: "Retire the sleep calls in the store tests", into: request, say: "The first words typed" },
  { look: text("Dispatch a job"), say: "The bar grows into the composer, holding what was typed" },
  { look: request, say: "Still in the field, the cursor after the words" },
  { look: button("Dispatch", { exact: true }), say: "Dispatch sends it" },
]);

export { dashboardQuickDispatch as "dashboard-quick-dispatch" };
