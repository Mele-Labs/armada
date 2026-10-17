// Everything the agent needs from him is one panel joined to the composer, the question first and what waits on him under it. A question that
// arrives has it up already. Esc folds it to its trigger on the box, and a press on the trigger raises it again. Over the
// `session-waiting-on-you` scenario. Plays under both takes: `?take=eyebrow` (a tab) and `?take=band` (a band on the box).

import { button, inside, region, role, walk } from "../walk";
import { toSessions } from "../sessions/walk-kit";

const sessions = region("Work", { exact: true });
const needs = region("Needs you");
const trigger = inside(needs, button("Needs you", { exact: true }));

const needsYouPanel = walk("session-waiting-on-you", [
  toSessions,
  { press: inside(sessions, button(/Pin the store clock/)), say: "A Session with a question and four things waiting" },
  { look: inside(needs, role("article", "Waiting on you")), say: "The panel is up already, the question first, joined to the box" },
  { key: "Escape", on: inside(needs, role("radio", "Fake")), say: "Esc folds it to its trigger" },
  { look: trigger, say: "The trigger sits on the box, a glyph for the question and one for each thing waiting" },
  { press: trigger, say: "A press raises the panel again" },
  { look: inside(needs, button("Open Approve #1843")), say: "What waits on him follows the question, under a line" },
]);

export { needsYouPanel as "needs-you-panel" };
