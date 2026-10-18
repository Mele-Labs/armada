import type { Guide } from "./guide";

/**
 * What a plan is, before any of the four guides about working one. Written
 * from `docs/concepts/session.md`, *The plan*.
 */
export const GUIDE_PLAN: Guide = {
  number: 17,
  group: "plan",
  title: "What is a plan?",
  piece: "plan.what",
  concept: "docs/concepts/session.md",
  steps: [
    "A plan is a job's statement of the work it means to do.",
    "The agent records it when planning ends, and you approve it in the thread.",
    "It holds groups in order, and each group holds tasks.",
    "A group names the subagent that takes it and the paths it touches.",
    "A task is open, working, handed in, done or failed.",
    "How far it got is Fleet's reading of what the job did, not the agent's word.",
    "Your changes go back as marks, and the agent answers them.",
    "The plan changes only when the agent records it again, as a new version.",
  ],
  // Under the line that names the groups, because the group is the relation.
  figure: { id: "group-order", at: 3 },
};
