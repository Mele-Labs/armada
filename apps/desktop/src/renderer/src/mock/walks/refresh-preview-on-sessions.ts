// A restart a Session hands the owner is the same key wherever it is drawn: its call card names it
// Refresh preview, and so does the Session's own list, where it was the raw command to copy. An
// ordinary command beside it keeps its block, with Run. Over `refresh-preview-on-sessions`.

import { button, inside, region, role, text, walk } from "../walk";
import { toSessions } from "../sessions/walk-kit";

const cockpit = region("Dashboard");
const needs = region("Needs you");
const preview = inside(needs, button("Refresh preview"));
const tests = inside(needs, role("group", "pnpm test"));

const refreshPreviewOnSessions = walk("refresh-preview-on-sessions", [
  { key: "w", on: cockpit, say: "w opens the first call" },
  { key: "l", on: cockpit, say: "Later, past the first call" },
  { key: "w", on: cockpit, say: "w opens the next" },
  { key: "l", on: cockpit, say: "Later, past the second" },
  { key: "w", on: cockpit, say: "w opens the next" },
  { look: button(/Refresh preview/), say: "On the call card the restart is one key, Refresh preview" },
  toSessions,
  { press: inside(region("Sessions"), button(/Pin the store clock/)), say: "Opened" },
  { press: inside(needs, button(/^Needs you/)), say: "Needs you, opened at the foot of the thread" },
  { look: preview, say: "The same item reads Refresh preview here too, with no command to copy" },
  { hover: preview, say: "Hovering names the command it stands for" },
  { look: inside(needs, text("pnpm test")), say: "An ordinary command beside it is still the command" },
  { look: inside(tests, button("Run")), say: "With Run, as before" },
  { press: preview, say: "Pressing it switches the build, once" },
  { look: text("Ran: exit 0"), say: "A clean exit settles the item and tells the agent, and only the command is left" },
]);

export { refreshPreviewOnSessions as "refresh-preview-on-sessions" };
