// A setup Command that fails stops the run: its row says the exit code and carries Try again and Fix with a Job, its output is in the Setup
// tab, and Try again runs setup on from that Command. Over the `run-setup-failed` scenario, where npm ci drops its connection the first time.

import { button, dialog, inside, region, role, text, walk } from "../walk";

const list = region("Jobs", { exact: true });
const confirm = dialog("Run auto-ynab");
const running = region("Run: auto-ynab");
const failedRow = inside(running, role("listitem", "frontend packages"));

const runSetupFailed = walk("run-setup-failed", [
  { press: inside(list, button("Run auto-ynab")), say: "auto-ynab has nothing installed" },
  { press: inside(confirm, button("Save and run")), say: "Save and run" },
  { look: inside(failedRow, text("exited with code 1")), say: "npm ci failed with code 1, so .venv and the processes never ran" },
  { look: inside(running, role("region", "Setup log")), say: "Its output is in the Setup tab" },
  { hover: inside(failedRow, button("Fix with a Job")), say: "Fix with a Job opens Dispatch with the run, the Command, its code, the last 40 lines and every log file" },
  { press: inside(failedRow, button("Try again")), say: "Try again runs setup on from npm ci, then the processes" },
  { look: inside(failedRow, role("img", "Done")), say: "npm ci goes through this time" },
  { look: inside(running, button("Open frontend")), say: "Setup done, both processes up" },
]);

export { runSetupFailed as "run-setup-failed" };
