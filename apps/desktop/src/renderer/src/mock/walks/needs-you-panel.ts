// Everything the agent needs from him is one panel joined to the composer, and it opens as a list: a row for each thing, the question first.
// A press on a row opens that one in the panel's place to answer or address; Back or Esc returns to the list, and Esc on the list folds the
// panel to its trigger. Over the `session-waiting-on-you` scenario.

import { button, inside, region, role, walk } from "../walk";
import { toSessions } from "../sessions/walk-kit";

const sessions = region("Jobs", { exact: true });
const needs = region("Needs you");
const trigger = inside(needs, button("Needs you", { exact: true }));
const question = inside(needs, button("Open Which clock?"));
const cargo = inside(needs, button("Open Run the store tests on your machine"));
const run = inside(needs, role("group", "cargo test -p store"));

const needsYouPanel = walk("session-waiting-on-you", [
  toSessions,
  { press: inside(sessions, button(/Pin the store clock/)), say: "A Job with a question and four things waiting" },
  { look: question, say: "The panel is up as a list, the question first, joined to the box" },
  { press: question, say: "A press on a row opens it" },
  { look: inside(needs, role("article", "Waiting on you")), say: "The question, to answer here" },
  { press: inside(needs, button("Back", { exact: true })), say: "Back returns to the list" },
  { look: cargo, say: "One row to a thing, whatever is waiting" },
  { press: cargo, say: "A command opens with its Run" },
  { look: inside(run, button("Run")), say: "Run, as it was in the thread" },
  { key: "Escape", on: inside(run, button("Run")), say: "Esc returns to the list" },
  { key: "Escape", on: cargo, say: "Esc on the list folds the panel" },
  { look: trigger, say: "The trigger sits on the box, a glyph for each thing waiting" },
]);

export { needsYouPanel as "needs-you-panel" };
