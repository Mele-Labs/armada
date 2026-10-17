// What waits on him is a row of pills hovering over the composer: Needs you, and the question when the agent asked one. A question that arrives
// has its sheet up over the thread already; its pill folds it. A press on a pill raises its sheet, another pill swaps it, and Esc puts it away.
// Over the `session-waiting-on-you` scenario.

import { button, inside, region, role, walk } from "../walk";
import { toSessions } from "../sessions/walk-kit";

const sessions = region("Work", { exact: true });
const needs = region("Needs you");
const needsPill = inside(needs, button(/^Needs you/));
const questionPill = inside(needs, button("Clock", { exact: true }));

const pillsOverComposer = walk("session-waiting-on-you", [
  toSessions,
  { press: inside(sessions, button(/Pin the store clock/)), say: "A Session with a question and four things waiting" },
  { look: inside(needs, role("article", "Waiting on you")), say: "The question's sheet is up over the thread already" },
  { press: questionPill, say: "Its pill folds the sheet away" },
  { look: needsPill, say: "Two pills hover over the composer, Needs you and the question" },
  { press: needsPill, say: "Needs you raises its sheet" },
  { press: questionPill, say: "The question's pill swaps it in" },
  { key: "Escape", on: questionPill, say: "Esc puts the sheet away, and focus stays on its pill" },
  { look: needsPill, say: "The pills stay where they were" },
]);

export { pillsOverComposer as "pills-over-composer" };
