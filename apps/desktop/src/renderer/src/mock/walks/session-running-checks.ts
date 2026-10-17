// The Checks a Session's agent has out, counted in its header: a press on the mark lists them, and a
// press on one opens that run over the Session.

import { button, dialog, inside, place, region, role, walk } from "../walk";

const MARK = button("2 checks running");
const LIST = role("menu", "2 checks running");

export const sessionRunningChecks = walk("session-running-checks", [
  { press: place("Sessions"), say: "Sessions" },
  { press: inside(region("Sessions"), button("Session running checks")), say: "A Session with two checks running" },
  { hover: MARK, say: "Checks running in this Session" },
  { press: MARK, say: "Press to list them" },
  { press: inside(LIST, role("menuitem", "Check test session_checking, running")), say: "Press one to open its log" },
  { look: dialog("Check log"), say: "Its log, over the Session" },
]);
