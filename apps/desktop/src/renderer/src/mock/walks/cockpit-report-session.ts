// Report, on the flight controls of an open Session: a dialog over the window, what he types and sends starts a Session in the same
// repository, told his words and what Bridge holds about the one he reported, and it opens on the stage. Over `cockpit-report-session`.

import { button, dialog, inside, region, role, text, walk } from "../walk";

const list = region("Jobs", { exact: true });
const controls = region("Flight controls");
const reporting = dialog("Report an issue with the Job");

const cockpitReportSession = walk("cockpit-report-session", [
  { press: inside(list, button("Pin the store clock")), say: "A Job open on the stage, its own flight controls under it" },
  { look: inside(controls, button("Report issue with job")), say: "Report is the fourth tile, a beetle, and the key 4. A Job that ended or went quiet can still be reported" },
  { press: inside(controls, button("Report issue with job")), say: "A dialog over the whole window, its box focused" },
  { look: inside(reporting, role("textbox", "Report")), say: "Empty, with no hint in it, and Send is off until there are words" },
  { type: "It pushed to main. I asked for a pull request.", into: inside(reporting, role("textbox", "Report")), say: "What went wrong, in his words" },
  { press: inside(reporting, button("Send")), say: "Send, or Enter. Esc or Cancel puts it away and keeps what was typed" },
  { look: region(/^Job: /), say: "A new Job is open on the stage, started in the same repository" },
  { look: text(/Armada job id: 6f2b9c1e-3d47-4a8b-9e10-5c7d2a4b8f31/), say: "His words first, untouched, then what Bridge holds about the Job, one labelled line each: its ids, transcript path, repository, branch and slot, state, model, context, the pull request, what it waits on him for, its last messages, tool calls and timeline entries" },
  { press: inside(region(/^Job: /), button("Back")), say: "Back to the map" },
]);

export { cockpitReportSession as "cockpit-report-session" };
