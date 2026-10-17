// A Session that split its plan into linked Sessions it follows: each group handed off carries its Session's row in Progress, and the same Sessions stand in the parent's Artifacts under Sessions.
// Over the `linked-sessions` scenario.

import { button, deckRow, inside, region, role, walk } from "../walk";

const plan = region("Progress", { exact: true });
const artifacts = region("Artifacts", { exact: true });
const controls = region("Flight controls");
const thread = role("region", "Thread", { exact: true });

export const linkedSessions = walk("linked-sessions", [
  { press: deckRow("Move the store to named migrations"), say: "The parent. In Work its linked Sessions stand under it. In Progress each group handed off carries the row of its Session" },
  { press: inside(controls, button("Artifacts")), say: "The same Sessions are in its Artifacts under Sessions: the title, then the group each came from, its state and its pull request" },
  { press: inside(artifacts, button("Back")), say: "Back returns to the thread" },
  { later: thread, say: "One of them asks a question and another merges its pull request" },
  { look: inside(thread, button(/^Migrations apply in order, Needs you/)), say: "Both are in the parent's thread where they happened, and its agent is told with each" },
  { press: inside(plan, button("List them in System")), say: "A group that was not handed off opens to Hand to a Session" },
  { key: "Enter", on: inside(plan, button("Open Migrations apply in order")), say: "Enter on a Session's row opens it" },
  { key: "Escape", on: region("Session: Migrations apply in order"), say: "Esc is Back. The parent returns with its thread and its open group as they were" },
]);
