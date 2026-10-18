// A Check the agent runs in its Session, seen from the thread. The thread holds a row per
// `armada check`: the key and a mark, pulsing while it is out.

import { button, inside, place, region, walk } from "../walk";

const THREAD = region("Thread");

export const sessionChecks = walk("session-checks", [
  { press: place("Cockpit"), say: "Cockpit" },
  { press: inside(region("Jobs", { exact: true }), button("Session check rows")), say: "A Job whose agent ran Checks" },
  { look: inside(THREAD, button("Check desktop:typecheck, passed")), say: "A Check that passed" },
  { look: inside(THREAD, button("Check components_test, failed")), say: "One that failed" },
  { hover: inside(THREAD, button("Check app_smoke, running")), say: "One still out: the mark pulses" },
  { later: inside(THREAD, button("Check app_smoke, running")), say: "It ends" },
  { look: inside(THREAD, button("Check app_smoke, failed")), say: "It failed" },
]);
