// The Checks a Session's agent has out, counted in its header: a press on the mark lists them.

import { button, inside, place, region, role, walk } from "../walk";

const MARK = button("2 checks running");
const LIST = role("menu", "2 checks running");

export const sessionRunningChecks = walk("session-running-checks", [
  { press: place("Cockpit"), say: "Cockpit" },
  { press: inside(region("Jobs", { exact: true }), button("Session running checks")), say: "A Job with two checks running" },
  { hover: MARK, say: "Checks running in this Job" },
  { press: MARK, say: "Press to list them" },
  { look: inside(LIST, role("menuitem", "Check test session_checking, running")), say: "Each one still out" },
]);
