// A Check the agent runs in its Session, seen from the thread and from the Checks page. The thread
// holds a row per `armada check`: the key and a mark, pulsing while it is out. Pressing one goes to
// the Checks page with that run open, and the Session that ran it is the owner chip on its line.

import { button, dialog, inside, region, role, row, walk } from "../walk";

const THREAD = region("Thread");
const CHECKS = region("Checks");

export const sessionChecks = walk("session-checks", [
  { press: button("Sessions", { exact: true }), say: "Sessions" },
  { press: inside(region("Sessions"), button("Session check rows")), say: "A Session whose agent ran Checks" },
  { look: inside(THREAD, button("Check desktop:typecheck, passed")), say: "A Check that passed" },
  { look: inside(THREAD, button("Check components_test, failed")), say: "One that failed" },
  { hover: inside(THREAD, button("Check app_smoke, running")), say: "One still out: the mark pulses" },
  { later: inside(THREAD, button("Check app_smoke, running")), say: "It ends" },
  { look: inside(THREAD, button("Check app_smoke, failed")), say: "It failed" },
  { press: inside(THREAD, button("Check app_smoke, failed")), say: "Pressing it goes to the Checks page" },
  { look: dialog("Check log"), say: "That run, open on its log" },
  { look: inside(inside(CHECKS, row(/app_smoke/)), role("button", "Worktree slot 10")), say: "The Session that ran it, as its owner chip" },
]);
