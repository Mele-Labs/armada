// A setup Command that fails stops the run: the panel names it, its output is in the Setup tab, and Restart runs setup again. Over the
// `run-setup-failed` scenario, where npm ci drops its connection the first time.

import { button, dialog, inside, region, role, walk } from "../walk";

const list = region("Jobs", { exact: true });
const confirm = dialog("Run auto-ynab");
const running = region("Run: auto-ynab");
const failed = inside(running, role("group", "Setup failed"));

const runSetupFailed = walk("run-setup-failed", [
  { press: inside(list, button("Run auto-ynab")), say: "auto-ynab has nothing installed" },
  { press: inside(confirm, button("Save and run")), say: "Save and run" },
  { look: failed, say: "npm ci in frontend exited with code 1, so the processes never started" },
  { look: inside(running, role("region", "Setup log")), say: "Its output is in the Setup tab" },
  { press: inside(failed, button("Restart")), say: "Restart runs setup again" },
  { look: inside(inside(running, role("listitem", "frontend packages")), role("img", "Done")), say: "npm ci goes through this time" },
  { look: inside(running, button("Open frontend")), say: "Setup done, both processes up" },
]);

export { runSetupFailed as "run-setup-failed" };
