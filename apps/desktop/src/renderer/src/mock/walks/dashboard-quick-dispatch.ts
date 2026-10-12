// The Dashboard with nothing waiting on the owner, and the title bar's one bar: n opens it ready for
// the work, the words typed lead with a row that dispatches them, and ⇧↵ opens the full composer
// holding them. Over `dashboard-quick-dispatch`.

import { role, text, walk } from "../walk";

const bar = role("combobox");

const dashboardQuickDispatch = walk("dashboard-quick-dispatch", [
  { look: role("button", /^Search, or describe work to dispatch/), say: "One bar in the title row, for search and dispatch alike" },
  { key: "n", on: role("tab", "Your move"), say: "Press n" },
  { look: bar, say: "The bar opens where it sits, ready for the work" },
  { type: "Retire the sleep calls in the store tests", into: bar, say: "The first words typed" },
  { look: role("option", /^Dispatch “Retire the sleep calls/), say: "They lead as the work itself; ⌘↵ sends them whatever is picked" },
  { key: "Shift+Enter", on: bar, say: "Press ⇧↵ for the full composer, for a file, a sketch or more words" },
  { look: text("Dispatch a job"), say: "The composer opens holding what was typed" },
]);

export { dashboardQuickDispatch as "dashboard-quick-dispatch" };
