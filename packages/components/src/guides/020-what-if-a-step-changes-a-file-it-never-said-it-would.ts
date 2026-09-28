import type { Guide } from "./guide";

/**
 * What a file nobody declared costs a Job. Written from `docs/concepts/plan.md`,
 * *Distinct from the declared scope's "plan"*, and `docs/concepts/judge.md`.
 *
 * **The owner's own question, 28 September 2026**, on a Record row reading
 * `outside the plan`: *"what does outside the plan mean to the job and how does
 * it impact the job?"* The row now says who declared what; this is the half a
 * row cannot carry without teaching.
 */
export const GUIDE_DRIFT: Guide = {
  number: 20,
  group: "run",
  title: "What if a step changes a file it never said it would?",
  piece: "run.drift",
  concept: "docs/concepts/plan.md",
  steps: [
    "A step can declare the paths it means to touch before it starts work.",
    "Armada compares what git found on the branch against that declaration.",
    "A changed file the declaration does not cover is recorded and named.",
    "It is a mark and not a verdict, so no gate fails on it and no step stops.",
    "The Judge is given it, and weighs whether the work still did what was asked.",
    "So the cost is one more thing the Judge reads, not a Job in trouble.",
    "A step that declared no paths has nothing to compare, and records nothing.",
  ],
};
