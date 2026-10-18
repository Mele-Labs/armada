import type { Guide } from "./guide";

/**
 * The first question anybody has, and the one the first fourteen guides could
 * not answer: they were written by lifting sentences off screens, so every one
 * of them started halfway through.
 *
 * Written from `docs/concepts/session.md`, which this concept was called until
 * 17 October 2026, rather than from a surface. Nothing here is drawn anywhere,
 * which is why no `?` claims `job.what` yet.
 */
export const GUIDE_JOB: Guide = {
  number: 15,
  group: "job",
  title: "What is a job?",
  piece: "job.what",
  concept: "docs/concepts/session.md",
  steps: [
    "A job is an agent working on something you asked for, in Bridge or in a terminal.",
    "Fleet keeps it as one record: its thread, its slot, its branch and its pull requests.",
    "It writes in a worktree slot of its own, never in yours.",
    "Its plan says what it means to do, and you approve the plan before the work goes on.",
    "A workflow, where one fits, lays out the steps it follows.",
    "Fleet reads the job's progress from what it does, not from what it says.",
    "When the job needs you, the call is a card in its thread.",
  ],
  // Under the line that names the steps: the relation is a job running them in
  // order, which is the same drawing guide 19 leads with.
  figure: { id: "workflow-steps", at: 5 },
};
