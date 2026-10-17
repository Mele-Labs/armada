// A Session that split its plan into linked Sessions it follows, first take: each handed off group on the Plan carries its Session's row.
// Over the `linked-sessions-a` scenario.

import { button, deckRow, inside, region, role, walk } from "../walk";

const plan = region("Plan", { exact: true });
const thread = role("region", "Thread", { exact: true });

export const linkedSessionsA = walk("linked-sessions-a", [
  { press: deckRow("Move the store to named migrations"), say: "The parent. In Work its linked Sessions stand under it. On the Plan each group handed off carries the row of its Session" },
  { later: thread, say: "One of them asks a question and another merges its pull request" },
  { look: inside(thread, button(/^Migrations apply in order, Needs you/)), say: "Both are in the parent's thread where they happened, and its agent is told with each" },
  { press: inside(plan, button("List them in System")), say: "A group that was not handed off opens to Hand to a Session" },
  { key: "Enter", on: inside(plan, button("Open Migrations apply in order")), say: "Enter on a Session's row opens it" },
  { key: "Escape", on: region("Session: Migrations apply in order"), say: "Esc is Back. The parent returns with its thread and its open group as they were" },
]);
