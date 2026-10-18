import type { Guide } from "./guide";

/**
 * The only guide in its group, and what the group was missing. Guide 11 filed
 * here explained how to read the plan's graph; the graph moved to the Plan tab
 * on 25 September 2026 and the guide was retired with it.
 *
 * Written from `docs/concepts/session.md`, *The rail*.
 */
export const GUIDE_WORKFLOW: Guide = {
  number: 19,
  group: "workflow",
  title: "What is a workflow?",
  piece: "workflow.what",
  concept: "docs/concepts/workflow.md",
  steps: [
    "A workflow is the list of steps a job follows, in order.",
    "It is a soft rail: the steps are in the agent's prompt and in the view.",
    "Fleet reads which step the job is on from what it does, not from what it says.",
    "When a step is skipped, the agent gets one nudge. Nothing blocks it.",
    "The proposer picks a workflow for your request, or none.",
    "A job with none is a custom job.",
    "You can change the workflow while the job runs, and the agent is told at once.",
  ],
  // Under the line that names the order, the same drawing guide 15 carries.
  figure: { id: "workflow-steps", at: 1 },
};
