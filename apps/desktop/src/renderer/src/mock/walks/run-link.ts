// A process that expects another on a fixed port: auto-ynab's frontend proxies to its backend on 8731. The confirm shows the link Fleet found
// with Fix with a Job and Pin the port, a run saved without acting on it shows the link in its panel, and Fix with a Job opens Dispatch with
// the brief written. Over the `run-link` scenario.

import { button, dialog, inside, region, role, tab, walk } from "../walk";

const list = region("Jobs", { exact: true });
const confirm = dialog("Run auto-ynab");
const running = region("Run: auto-ynab");

const runLink = walk("run-link", [
  { press: inside(list, button("Run auto-ynab")), say: "auto-ynab has no saved run, so play asks first" },
  { hover: inside(confirm, role("img", /runs on a leased port/)), say: "frontend expects backend on 8731, found in vite.config.js" },
  { press: inside(confirm, button("Pin the port")), say: "Pin the port: backend's row shows 8731 and a pin where leased was" },
  { press: inside(confirm, button("Unpin backend")), say: "The pin on the row undoes it" },
  { press: inside(confirm, button("Save and run")), say: "Save and run goes ahead with neither act" },
  { press: inside(running, tab("frontend")), say: "frontend's log" },
  { look: inside(running, role("group", "frontend expects backend")), say: "The refused connection is in the log, and the frontend row has the link mark with the same two acts" },
  { press: inside(running, button("Fix with a Job")), say: "Dispatch opens with the brief written" },
]);

export { runLink as "run-link" };
