// A checkout with nothing installed: the confirm lists the setup Fleet drafted above the processes, and the run sets up before its processes
// start, setup's output under a tab of its own. Over the `run-setup` scenario.

import { button, dialog, inside, region, role, tab, walk } from "../walk";

const list = region("Jobs", { exact: true });
const confirm = dialog("Run auto-ynab");
const running = region("Run: auto-ynab");

const runSetup = walk("run-setup", [
  { press: inside(list, button("Run auto-ynab")), say: "auto-ynab has no saved run and nothing installed" },
  { look: inside(confirm, role("textbox", "Setup 1 command")), say: "Setup drafted from its files: npm ci in frontend, then .venv at the root" },
  { look: inside(confirm, role("textbox", "Process 2 command")), say: "Then backend and frontend, each on a leased port. All of it editable" },
  { press: inside(confirm, button("Save and run")), say: "Save and run" },
  { hover: inside(inside(running, role("listitem", "backend")), role("img", "Waiting for setup")), say: "The processes wait while setup runs, its output in the Setup tab" },
  { look: inside(inside(running, role("listitem", "frontend packages")), role("img", "Done")), say: "npm ci done, .venv next" },
  { look: inside(running, button("Open frontend")), say: "Setup done, both processes up" },
  { press: inside(running, tab("backend")), say: "Each process keeps its own log beside Setup" },
]);

export { runSetup as "run-setup" };
