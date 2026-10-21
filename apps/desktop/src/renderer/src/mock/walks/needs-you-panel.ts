// Everything the agent needs from him is one panel joined to the composer, and it opens as a list: Questions, then Walks, Commands and PRs.
// A press on a question opens it in the panel's place to answer; Back or Esc returns to the list. Anything else is done in one press where it
// stands. Esc on the list folds the panel to its trigger. Over the `session-waiting-on-you` scenario.

import { button, inside, region, role, walk } from "../walk";
import { toSessions } from "../sessions/walk-kit";

const sessions = region("Jobs", { exact: true });
const needs = region("Needs you");
const trigger = inside(needs, button("Needs you", { exact: true }));
const questions = inside(needs, region("Questions"));
const commands = inside(needs, region("Commands"));
const question = inside(questions, button("Open Which clock?"));
const askedInWindow = inside(questions, button("Open Should the count sit beside the title or under it?"));
const run = inside(commands, role("group", "cargo test -p store"));

const needsYouPanel = walk("session-waiting-on-you", [
  toSessions,
  { press: inside(sessions, button(/Pin the store clock/)), say: "A Job with a question and four things waiting" },
  { look: questions, say: "Questions first" },
  { look: inside(askedInWindow, role("img", "Question")), say: "A question asked in the walk window is marked as a question" },
  { look: commands, say: "Then Walks, Commands and PRs" },
  { press: question, say: "A question opens in the panel" },
  { look: inside(needs, role("article", "Waiting on you")), say: "Answered here" },
  { press: inside(needs, button("Back", { exact: true })), say: "Back returns to the list" },
  { look: inside(run, button("Run")), say: "A command runs from the list in one press" },
  { key: "Escape", on: inside(run, button("Run")), say: "Esc folds the panel" },
  { look: trigger, say: "The trigger, a glyph for each thing waiting" },
]);

export { needsYouPanel as "needs-you-panel" };
