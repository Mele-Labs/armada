// A restart a Session hands the owner is the same key wherever it is drawn: its call strip names it
// Refresh preview, and so does the Session's own list, where it was the raw command to copy. An
// ordinary command beside it keeps its block, with Run. Over `refresh-preview-on-sessions`.

import { button, inside, region, role, text, walk } from "../walk";
import { toSessions } from "../sessions/walk-kit";

const needs = region("Needs you");
const preview = inside(needs, button("Refresh preview"));
const tests = inside(needs, role("group", "pnpm test"));

const refreshPreviewOnSessions = walk("refresh-preview-on-sessions", [
  { press: inside(region("Incoming calls"), button(/Pin the store clock/)), say: "A Session's call opens the Session, and leaves Incoming calls for the thread" },
  { press: inside(needs, button(/^Needs you/)), say: "Needs you, opened at the foot of the thread" },
  { look: preview, say: "In the thread the restart is one key, Refresh preview" },
  toSessions,
  { press: inside(region("Work", { exact: true }), button(/Pin the store clock/)), say: "Opened, the band at the foot of the thread as it was left" },
  { look: preview, say: "The same item reads Refresh preview here too, with no command to copy" },
  { hover: preview, say: "Hovering names the command it stands for" },
  { look: tests, say: "An ordinary command beside it is still the command" },
  { look: inside(tests, button("Run")), say: "With Run, as before" },
  { press: preview, say: "Pressing it switches the build, once" },
  { look: text("It ran clean."), say: "A clean exit settles the item and tells the agent, and only the command is left" },
]);

export { refreshPreviewOnSessions as "refresh-preview-on-sessions" };
