// A Session that split its plan into linked Sessions it follows, second take: a Linked section in the parent's Ledger.
// Over the `linked-sessions-b` scenario.

import { button, deckRow, inside, region, role, walk } from "../walk";

const plan = region("Plan", { exact: true });
const ledger = region("Ledger", { exact: true });
const thread = role("region", "Thread", { exact: true });

export const linkedSessionsB = walk("linked-sessions-b", [
  { press: deckRow("Move the store to named migrations"), say: "The parent. In Work its linked Sessions stand under it. In its Ledger they are under Linked, each with the group it came from" },
  { later: thread, say: "One of them asks a question and another merges its pull request" },
  { look: inside(thread, button(/^Migrations apply in order, Needs you/)), say: "Both are in the parent's thread where they happened, and its agent is told with each" },
  { press: inside(plan, button("List them in System")), say: "A group that was not handed off opens to Hand to a Session" },
  { key: "Enter", on: inside(ledger, button("Open Migrations apply in order")), say: "Enter on a Session's row opens it" },
  { key: "Escape", on: region("Session: Migrations apply in order"), say: "Esc is Back. The parent returns with its thread and its open group as they were" },
]);
