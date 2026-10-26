// The running panel from first press to a run that is up: play on auto-ynab, which has no saved run and nothing installed, opens the panel
// on Fleet's draft with its setup, its processes and a link it found. Save and run sets up, `npm ci` fails, the failed row and the Setup log
// say so, Edit the run turns the rows into fields where they are, and Save and restart picks setup up at the Command that failed. Over the
// `run-app` scenario.

import { button, inside, region, role, tab, text, walk } from "../walk";

const list = region("Jobs", { exact: true });
const pane = region("Run: auto-ynab");
const failedRow = inside(pane, role("listitem", "frontend packages"));

const runApp = walk("run-app", [
  { press: inside(list, button("Run auto-ynab")), say: "auto-ynab has no saved run and nothing installed, so play opens the panel editing Fleet's draft" },
  { look: inside(pane, role("textbox", "Setup 1 command")), say: "Setup drafted from its files, above the processes: npm ci in frontend, then .venv at the root" },
  { look: inside(pane, role("textbox", "Process 2 command")), say: "backend and frontend, each on a leased port" },
  { hover: inside(pane, role("img", /runs on a leased port/)), say: "frontend expects backend on 8731, found in vite.config.js" },
  { press: inside(pane, button("Save and run")), say: "Save and run" },
  { hover: inside(inside(pane, role("listitem", "backend")), role("img", "Waiting for setup")), say: "The processes wait while setup runs, its output in the Setup tab" },
  { look: inside(failedRow, text("exited with code 1")), say: "npm ci failed with code 1, so .venv and the processes never ran" },
  { look: inside(pane, role("region", "Setup log")), say: "The failure is in the Setup log" },
  { hover: inside(failedRow, button("Fix with a Job")), say: "Fix with a Job opens Dispatch with the run, the Command, its code, the last 40 lines and every log file" },
  { key: "e", on: pane, say: "Edit the run turns the same rows into fields, in place" },
  { type: "npm ci --prefer-offline", into: inside(failedRow, role("textbox", "Setup 1 command")), say: "The Command changes beside its exit code and the output that failed" },
  { press: inside(pane, button("Save and restart")), say: "Save and restart" },
  { look: inside(failedRow, role("img", "Done")), say: "Setup picks up at npm ci and it goes through" },
  { look: inside(pane, button("Open frontend")), say: "Setup done, both processes up" },
  { press: inside(pane, tab("backend")), say: "Each process has its own log" },
]);

export { runApp as "run-app" };
