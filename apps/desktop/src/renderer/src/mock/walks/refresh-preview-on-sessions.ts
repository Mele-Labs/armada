// A restart a Session hands the owner is the same key wherever it is drawn: its call strip names it
// Refresh preview, and so does the Session's own list, where it was the raw command to copy. An
// ordinary command beside it keeps its block, with Run. Over `refresh-preview-on-sessions`.

import { button, deckRow, inside, region, role, text, walk } from "../walk";
import { toSessions } from "../sessions/walk-kit";

const needs = region("Needs you");
const preview = inside(needs, button("Refresh preview"));
const tests = inside(needs, role("group", "pnpm test"));

const refreshPreviewOnSessions = walk("refresh-preview-on-sessions", [
  { press: deckRow(/Pin the store clock/), say: "A Job's row opens the Job, and its call is in the thread" },
  { press: inside(needs, button(/^Needs you/)), say: "Needs you, raised from the composer" },
  { look: preview, say: "The restart is a row in the list, Refresh preview" },
  toSessions,
  { press: inside(region("Jobs", { exact: true }), button(/Pin the store clock/)), say: "Opened, the Needs you panel as it was left" },
  { look: preview, say: "The same item reads Refresh preview here too, with no command to copy" },
  { press: preview, say: "A press on its row opens it" },
  { hover: preview, say: "Hovering names the command it stands for" },
  { press: inside(needs, button("Back", { exact: true })), say: "Back to the list" },
  { press: inside(needs, button("Open Run the tests")), say: "An ordinary command beside it" },
  { look: tests, say: "Is still the command" },
  { look: inside(tests, button("Run")), say: "With Run, as before" },
  { press: inside(needs, button("Back", { exact: true })), say: "Back to the list" },
  { press: preview, say: "Opened again" },
  { press: preview, say: "Pressing it switches the build, once" },
  { look: text("It ran clean."), say: "A clean exit settles the item and tells the agent, and only the command is left" },
]);

export { refreshPreviewOnSessions as "refresh-preview-on-sessions" };
