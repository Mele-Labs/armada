// A process that expects another on a fixed port: auto-ynab's frontend proxies to its backend on 8731. The panel's draft shows the link Fleet
// found with Fix with a Job and Pin the port, a run saved without acting on it keeps the link on its running row, and Fix with a Job opens
// Dispatch with the brief written. Over the `run-link` scenario.

import { button, inside, region, role, tab, walk } from "../walk";

const list = region("Jobs", { exact: true });
const pane = region("Run: auto-ynab");

const runLink = walk("run-link", [
  { press: inside(list, button("Run auto-ynab")), say: "auto-ynab has no saved run, so play opens the panel on Fleet's draft" },
  { hover: inside(pane, role("img", /runs on a leased port/)), say: "frontend expects backend on 8731, found in vite.config.js" },
  { press: inside(pane, button("Pin the port")), say: "Pin the port: backend's row shows 8731 and a pin where leased was" },
  { press: inside(pane, button("Unpin backend")), say: "The pin on the row undoes it" },
  { press: inside(pane, button("Save and run")), say: "Save and run goes ahead with neither act" },
  { press: inside(pane, tab("frontend")), say: "frontend's log" },
  { look: inside(pane, role("group", "frontend expects backend")), say: "The refused connection is in the log, and the frontend row keeps the link mark with the same two acts" },
  { press: inside(pane, button("Fix with a Job")), say: "Dispatch opens with the brief written" },
]);

export { runLink as "run-link" };
