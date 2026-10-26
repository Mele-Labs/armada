// Running an app for the first time from the Cockpit: play on auto-ynab, which has no saved run, opens the running panel on the run Fleet
// drafted, already editing; Save and run starts it in the same panel, which goes from starting to ready with a log for each process. Over
// the `run-an-app` scenario.

import { button, inside, region, role, tab, text, walk } from "../walk";

const list = region("Jobs", { exact: true });
const pane = region("Run: auto-ynab");

const runAnApp = walk("run-an-app", [
  { press: inside(list, button("Run auto-ynab")), say: "auto-ynab has no saved run, so play opens the panel on Fleet's draft" },
  { look: inside(pane, role("textbox", "Run name")), say: "The run's name in the head, ready to change" },
  { look: inside(pane, role("textbox", "Process 2 command")), say: "Each process's name, folder and command in its row, the full path under the folder" },
  { look: inside(pane, text("this Mac")), say: "Where it saves, beside Cancel and Save and run" },
  { press: inside(pane, button("Save and run")), say: "Save and run" },
  { look: inside(pane, button("Open frontend")), say: "The same panel runs it; both processes come up and Open appears" },
  { press: inside(pane, tab("frontend")), say: "Each process has its own log" },
]);

export { runAnApp as "run-an-app" };
