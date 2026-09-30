import type { Guide } from "./guide";

/**
 * The line that used to stand over the group controls on the Plan board
 * (`ASKS_SAY`, `#1552`). Since 30 Sep 2026 the edits are a person's own, made
 * through Fleet, and Propose a change is the one request.
 */
export const GUIDE_PLAN_ASKS: Guide = {
  number: 6,
  group: "plan",
  title: "How do I change the plan?",
  piece: "plan.asks",
  concept: "docs/concepts/plan.md",
  steps: [
    "The plan is the Drone's record of how it means to work.",
    "Add task, Edit this task, Drop this task and Remove change it directly, through Fleet.",
    "Drag a group or a task to move it, or press ⌥↑ or ⌥↓ on it.",
    "Propose a change is a request. It reaches the Drone that wrote the plan, and it may refuse.",
    "A refusal is drawn under the ask, in the Drone's own words, and the plan below stays as it was.",
    "Moving, Remove and Propose a change are offered while the step that recorded the plan is waiting on a person.",
    "Past that gate the plan is a record of what was run.",
    "Two groups claiming one file have an order between them their own scopes decide.",
    "A move can reverse it. The group that shares a file says so on its card.",
  ],
};
