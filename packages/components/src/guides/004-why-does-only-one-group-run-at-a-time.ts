import type { Guide } from "./guide";

/**
 * The rule no screen states any more (`#1530`). The implement board carried
 * one sentence of it at a failed boundary — *No task of group 4 starts until
 * this boundary passes* — and the owner cut it on 28 Sep. It is true of a job
 * that has never run, which is the test that makes it a guide rather than a
 * line on the screen. What the boards still say is where each group got to,
 * which is this job's own fact.
 */
export const GUIDE_GROUP_ORDER: Guide = {
  number: 4,
  group: "plan",
  title: "Why does only one group run at a time?",
  piece: "plan.group-order",
  concept: "docs/concepts/plan.md",
  steps: [
    "A step's plan is a list of tasks, gathered into groups.",
    "The groups run one at a time, in plan order.",
    "Every task in a group is worked before that group is checked.",
    "No task of the next group starts while this one is being checked.",
    "Groups exist because a parallel schedule cannot be worked out from the tasks alone.",
    "Two tasks that write one file are found by comparing their paths.",
    "One task using what another made leaves no path to compare, and plan order carries it.",
    "A task reads done only once its group has gone green.",
    "Until then its own mark says where it got to and nothing more.",
    "The order is written before the work starts and holds while the step runs.",
  ],
  // Under the line that names the order, which is the relation the figure draws.
  figure: { id: "group-order", at: 2 },
};
