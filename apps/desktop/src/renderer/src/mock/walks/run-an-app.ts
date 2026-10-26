// Running an app from the Cockpit: the caret on armada lists its saved runs, play on auto-ynab asks first and then runs it, the
// panel goes from starting to ready with a log for each process, and the same pair in a Job's header runs the Job's own bay. Over the
// `run-an-app` scenario.

import { button, dialog, inside, region, role, tab, text, walk } from "../walk";

const list = region("Jobs", { exact: true });
const confirm = dialog("Run auto-ynab");
const running = region("Run: auto-ynab");
const onBay = region("Run: Mock Bridge");

const runAnApp = walk("run-an-app", [
  { press: inside(list, button("Runs for armada")), say: "The caret beside armada's play glyph lists its saved runs" },
  { look: role("menuitem", "Bridge dev, scratch Fleet"), say: "The runs, and Add a way to run last" },
  { key: "Escape", on: role("menuitem", "Add a way to run"), say: "Esc closes it" },
  { press: inside(list, button("Run auto-ynab")), say: "auto-ynab has no saved run, so play asks first" },
  { look: inside(confirm, text("this Mac")), say: "The run Fleet drafted, each process editable, and where it is saved" },
  { press: inside(confirm, button("Save and run")), say: "Save and run" },
  { look: inside(running, button("Open frontend")), say: "The panel opens on the run; both processes come up and Open appears" },
  { press: inside(running, tab("frontend")), say: "Each process has its own log" },
  { press: inside(running, button("Stop", { exact: true })), say: "Stop" },
  { look: inside(running, role("img", "Exited")), say: "Both processes stopped" },
  { press: inside(list, button(/Pin the store clock/)), say: "Open the Job" },
  { press: button("Run this Job"), say: "Its play glyph runs the Job's bay" },
  { look: inside(onBay, text("Pin the store clock")), say: "The panel names the Job and its bay, not main" },
  { key: "Escape", on: onBay, say: "Esc goes back to the Job, where it was left" },
]);

export { runAnApp as "run-an-app" };
