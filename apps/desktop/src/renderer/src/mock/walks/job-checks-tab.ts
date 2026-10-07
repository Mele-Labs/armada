// A Job's Checks tab, after Record, and the filter row it shares with the Checks page: the Job's
// Checks newest first, from its gate, a Drone and the merge line, narrowed by state. A mock, no
// Fleet behind it.

import { button, inside, role, tab, text, walk } from "../walk";

const STRIP = role("tablist", "Job detail");
const PANEL = role("dialog", "Check");
const RAIL = role("navigation", "Work");

export const jobChecksTab = walk("job-checks", [
  { look: inside(STRIP, tab("Checks")), say: "Checks, after Record" },
  { press: inside(STRIP, tab("Checks")), say: "This Job's Checks" },
  { look: button("hooks_test", { exact: true }), say: "Newest first, with who asked for each" },
  { hover: text("running"), say: "Out now" },
  { press: tab("Active"), say: "Active: running" },
  { press: tab("Waiting"), say: "Waiting: asked for, not started" },
  { press: tab("Completed"), say: "Completed: passed, failed and skipped" },
  { press: tab("Passed"), say: "Passed" },
  { press: tab("Failed"), say: "Failed" },
  { press: tab("Skipped"), say: "Skipped" },
  { press: tab("All"), say: "All, as it opened" },
  { press: button("format", { exact: true }), say: "A Check a Drone asked for" },
  { look: inside(PANEL, text(/Diff in crates/)), say: "Its facts over its log" },
  { press: inside(PANEL, button(/Drone/)), say: "Requested by: that Drone, opened within this Job" },
  { look: inside(STRIP, tab("Drones")), say: "The Drones tab, still this Job" },
  { press: inside(RAIL, button("Checks", { exact: true })), say: "The Checks page" },
  { look: tab("Skipped"), say: "The same filter row" },
  { press: tab("Failed"), say: "Every Job's failed Checks, and the checkout's" },
]);
