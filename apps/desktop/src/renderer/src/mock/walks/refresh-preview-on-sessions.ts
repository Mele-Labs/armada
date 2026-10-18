// A restart a Session hands the owner is the same key wherever it is drawn: its call strip names it
// Refresh preview, and so does the Session's Needs you band, one press folded or open. An ordinary
// command keeps its block, with Run, under Commands. Over `refresh-preview-on-sessions`.

import { button, deckRow, inside, region, role, text, walk } from "../walk";
import { toSessions } from "../sessions/walk-kit";

const needs = region("Needs you");
const preview = inside(needs, button("Refresh preview"));
const tests = inside(needs, role("group", "pnpm test"));

const refreshPreviewOnSessions = walk("refresh-preview-on-sessions", [
  { press: deckRow(/Pin the store clock/), say: "A Job's row opens the Job, and its call is in the thread" },
  { look: preview, say: "Refresh preview is a key on the Needs you band" },
  toSessions,
  { press: inside(region("Jobs", { exact: true }), button(/Pin the store clock/)), say: "The same Job on the Jobs page" },
  { hover: preview, say: "Hovering names the command it stands for" },
  { press: inside(needs, button("Needs you", { exact: true })), say: "The panel opens" },
  { look: inside(tests, button("Run")), say: "An ordinary command, under Commands, runs from the list" },
  { press: preview, say: "One press on the band switches the build" },
  { look: text("It ran clean."), say: "A clean exit settles the item and tells the agent" },
]);

export { refreshPreviewOnSessions as "refresh-preview-on-sessions" };
