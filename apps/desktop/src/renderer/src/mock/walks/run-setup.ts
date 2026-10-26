// A checkout with nothing installed: the panel's draft lists the setup Fleet drafted above the processes, and the run sets up before its
// processes start, setup's output under a tab of its own. Over the `run-setup` scenario.

import { button, inside, region, role, tab, walk } from "../walk";

const list = region("Jobs", { exact: true });
const pane = region("Run: auto-ynab");

const runSetup = walk("run-setup", [
  { press: inside(list, button("Run auto-ynab")), say: "auto-ynab has no saved run and nothing installed" },
  { look: inside(pane, role("textbox", "Setup 1 command")), say: "Setup drafted from its files, above the processes: npm ci in frontend, then .venv at the root" },
  { press: inside(pane, button("Save and run")), say: "Save and run" },
  { hover: inside(inside(pane, role("listitem", "backend")), role("img", "Waiting for setup")), say: "The processes wait while setup runs, its output in the Setup tab" },
  { look: inside(inside(pane, role("listitem", "frontend packages")), role("img", "Done")), say: "npm ci done, .venv next" },
  { look: inside(pane, button("Open frontend")), say: "Setup done, both processes up" },
  { press: inside(pane, tab("backend")), say: "Each process keeps its own log beside Setup" },
]);

export { runSetup as "run-setup" };
