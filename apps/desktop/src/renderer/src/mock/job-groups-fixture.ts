// A feature Job whose plan Fleet ran in groups, **in the shape Fleet serves it
// since 23.4** (spike 022, slice 2): three groups on `WorkPlan.groups`, the
// first two passed with the commit each left, the third red on all three of
// the runs `implement`'s retries allow, so its task reads failed and the step
// stopped for a person. Every Check run names the group and run it held back.

import type { CheckRun, PlanGroup, PlanTask, StepDetail } from "@armada/protocol";
import type { JobFixture } from "@armada/screens/src/fixtures/fixture";

import { featureOnItsPlan } from "./job-detail-fixtures";

const AT = (minute: number) => `2026-10-02T14:${String(minute).padStart(2, "0")}:00.000Z`;

/** Why T4 failed, as Fleet writes it. */
const T4_FAILED =
  "G3's Checks were still red on run 3, the last its retries allow: test";

const TASKS: PlanTask[] = [
  { id: "T1", title: "Draw a group's runs on its card", scope: ["packages/screens/src/plan-board.ts"], state: "done", group: "G1" },
  { id: "T2", title: "Name the group on a Check run", scope: ["packages/screens/src/record-check.tsx"], state: "done", group: "G1" },
  { id: "T3", title: "Read Fleet's groups in the draft", scope: ["packages/screens/src/draft/group.ts"], state: "done", group: "G2" },
  {
    id: "T4",
    title: "Answer restart and move in the mock",
    scope: ["packages/screens/src/plan-review.tsx"],
    state: "failed",
    group: "G3",
    failed_reason: T4_FAILED,
  },
];

const red = { named: "failed", trigger: "gate_failure" } as const;

const GROUPS: PlanGroup[] = [
  {
    id: "G1",
    tasks: ["T1", "T2"],
    state: "passed",
    started_at: AT(1),
    ended_at: AT(9),
    attempts: [
      {
        attempt: 1,
        step_id: "implement",
        step_attempt: 1,
        started_at: AT(1),
        ended_at: AT(9),
        verdict: { attempt: 1, named: "passed" },
        commit: "4c1b9d2",
      },
    ],
  },
  {
    id: "G2",
    tasks: ["T3"],
    state: "passed",
    started_at: AT(10),
    ended_at: AT(15),
    attempts: [
      {
        attempt: 1,
        step_id: "implement",
        step_attempt: 1,
        started_at: AT(10),
        ended_at: AT(15),
        verdict: { attempt: 1, named: "passed" },
        commit: "7a2f0c5",
      },
    ],
  },
  {
    id: "G3",
    tasks: ["T4"],
    state: "failed",
    started_at: AT(16),
    ended_at: AT(30),
    attempts: [1, 2, 3].map((run) => ({
      attempt: run,
      step_id: "implement",
      step_attempt: run,
      started_at: AT(12 + run * 4),
      ended_at: AT(15 + run * 4),
      verdict: { attempt: run, ...red },
    })),
  },
];

/** The Checks one run of a group ran: `typecheck` green, and `test` as given. */
function ranAt(group: string, run: number, stepAttempt: number, test: "passed" | "failed"): CheckRun[] {
  const at = { attempt: stepAttempt, group, group_attempt: run };
  return [
    { ...at, name: "typecheck", outcome: "passed" },
    test === "passed"
      ? { ...at, name: "test", outcome: "passed" }
      : { ...at, name: "test", outcome: "failed", expected: "exits 0", produced: "exited 1 — 2 of 1104 failed" },
  ];
}

/** The feature Job of `featureOnItsPlan`, its plan run group by group. */
export function featureRunInGroups(): JobFixture {
  const base = featureOnItsPlan();
  if (base.watched.state !== "read") return base;
  const whole = base.watched.detail;
  const steps: StepDetail[] = whole.steps.map((step) => {
    if (step.step_id === "plan") {
      return { ...step, state: "advanced", attempts: [{ attempt: 1, outcome: "advanced", started_at: AT(0), ended_at: AT(1) }] };
    }
    if (step.step_id !== "implement") return step;
    return {
      ...step,
      state: "stopped",
      last_verdict: { attempt: 3, ...red },
      attempts: [
        { attempt: 1, outcome: "retrying", why: "gate_failure", started_at: AT(1), ended_at: AT(19) },
        { attempt: 2, outcome: "retrying", why: "gate_failure", started_at: AT(20), ended_at: AT(23) },
        { attempt: 3, outcome: "stopped", why: "gate_failure", started_at: AT(24), ended_at: AT(30) },
      ],
      check_runs: [
        ...ranAt("G1", 1, 1, "passed"),
        ...ranAt("G2", 1, 1, "passed"),
        ...ranAt("G3", 1, 1, "failed"),
        ...ranAt("G3", 2, 2, "failed"),
        ...ranAt("G3", 3, 3, "failed"),
      ],
      entered_at: AT(1),
      updated_at: AT(30),
    };
  });
  const job = {
    ...base.job,
    title: "Draw the plan's groups as Fleet runs them",
    status: "awaiting_repair",
    current_step_id: "implement",
    tasks: { done: 3, working: 0, open: 0, dropped: 0, failed: 1 },
  };
  const detail = {
    ...whole,
    job,
    steps,
    work_plan: {
      ...whole.work_plan!,
      approach: "Read Fleet's groups, name each Check run's group, then answer the two acts.",
      tasks: TASKS,
      groups: GROUPS,
    },
  };
  return { ...base, job, watched: { ...base.watched, detail } };
}
