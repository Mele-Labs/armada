import type { Guide } from "./guide";

/**
 * Land's mark, beside the word for where the work got to. It rode on the
 * sentence naming this job's rule — *Completes when its pull request lands* —
 * until that sentence left a finished job (owner, 1 Oct 2026): Land draws only
 * a job that completed, so the rule had nothing left to say there. What a
 * landing rule is, and that there are four of them, is here.
 *
 * **No figure.** Four rules and a distinction; nothing relates to anything,
 * and there is no honest picture of a rule.
 */
export const GUIDE_COMPLETION: Guide = {
  number: 1,
  group: "job",
  title: "When is a job done?",
  piece: "land.completion",
  concept: "docs/concepts/landing.md",
  steps: [
    "A job is done when its landing rule is met.",
    "The rule is chosen when the job is dispatched, and frozen from then on.",
    "Nothing about it moves while the job runs.",
    "There are four rules: the pull request merged, the pull request opened, every member landed, " +
      "and the delivering step delivered.",
    "Done is not the same as ended.",
    "A job that was killed, rejected or escalated ended without its rule being met.",
    "Land still draws what it left behind.",
  ],
};
