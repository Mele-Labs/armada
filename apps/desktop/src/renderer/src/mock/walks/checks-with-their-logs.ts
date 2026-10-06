// The Checks page, a rail row of its own: every Check this repository has had requested or run,
// each with a mark for its state, and one opened beside its facts and its log. The one still out
// is followed as it prints. A mock, no Fleet behind it.

import { button, inside, role, text, walk } from "../walk";

const PANEL = role("dialog", "Check");

export const checksWithTheirLogs = walk("checks", [
  { press: button("Checks", { exact: true }), say: "Checks, from the rail" },
  { look: role("region", "Checks"), say: "A row for each Check asked for or run" },
  { hover: text("running"), say: "Out now: the pulsing mark" },
  { hover: text("waiting"), say: "Asked for in Verify, not started" },
  { press: button("test", { exact: true }), say: "The Check that is out" },
  { look: inside(PANEL, text("Command")), say: "Its facts over its log" },
  { look: inside(PANEL, text(/reads_the_runtime_file/)), say: "Its log, followed as it prints" },
  { press: inside(PANEL, button("Close")), say: "Close" },
  { press: button("typecheck", { exact: true }), say: "One that failed" },
  { look: inside(PANEL, text("Exit")), say: "Its exit code against the one expected" },
  { look: inside(PANEL, text(/TS2322/)), say: "Its log, read whole" },
  { look: inside(PANEL, text("Started outside a Job")), say: "Requested by: a run started outside any Job, as text" },
  { press: inside(PANEL, button("Close")), say: "Close" },
  { press: button("bridge_test", { exact: true }), say: "One that passed and changed a file" },
  { look: inside(PANEL, text("Changed")), say: "The file it changed" },
  { press: inside(PANEL, button("Close")), say: "Close" },
  { press: button("desktop_test", { exact: true }), say: "A Check the merge line is running for a branch" },
  { look: inside(PANEL, text("fleet/pulse-log-rows")), say: "Its branch, with its log followed" },
  { look: inside(PANEL, button("Merge line")), say: "Requested by: the Merge line, a link to it" },
  { press: inside(PANEL, button("Merge line")), say: "The link opens the Merge line" },
]);
