// A Check the agent's Session has out, opened from its thread: the log grows while it runs, and the
// panel's head holds Stop and Force stop for as long as it does.

import { button, dialog, inside, place, region, walk } from "../walk";

const THREAD = region("Thread");
const LOG = dialog("Check log");

export const sessionCheckLiveLog = walk("session-check-live-log", [
  { press: place("Sessions"), say: "Sessions" },
  { press: inside(region("Sessions"), button("Session check rows")), say: "A Session whose agent has a Check out" },
  { press: inside(THREAD, button("Check app_smoke, running")), say: "Pressing the one still out opens its log" },
  { look: inside(LOG, button("Stop")), say: "Stop asks it to wind down" },
  { look: inside(LOG, button("Force stop")), say: "Force stop ends it at once" },
  { look: LOG, say: "Its lines arrive as it prints" },
]);
