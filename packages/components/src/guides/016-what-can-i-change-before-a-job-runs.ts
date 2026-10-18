import type { Guide } from "./guide";

/**
 * What a person can change on a job, gathered in one place. Read off
 * the decision of 15 October 2026 that Sessions take what Jobs did well, so a
 * control added to the job's panels makes this stale. The file and the piece keep the name
 * they had when this was asked before a job started.
 */
export const GUIDE_BEFORE_A_RUN: Guide = {
  number: 16,
  group: "job",
  title: "What can I change on a job?",
  piece: "job.before",
  concept: "docs/concepts/job-proposer.md",
  steps: [
    "The proposer reads your request and fills in the name, the workflow, the model and the effort.",
    "A job starts as soon as the proposal lands.",
    "The workflow, the model and the effort stay yours to change while it runs.",
    "A change to the workflow or the plan reaches the agent at once, or wakes it if it is idle.",
    "A change of model or effort applies from the next turn, and the agent is told.",
    "Each change shows as a row in the thread.",
    "A setting turns workflows off, so every job is dispatched plain.",
  ],
};
