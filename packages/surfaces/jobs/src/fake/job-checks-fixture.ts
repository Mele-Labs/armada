// A feature Job whose one group's gate is running its Checks, in the shape Fleet
// served Job 13 on 7 Oct 2026: two of the declared Checks cover paths nothing
// changed under, so Fleet recorded them `skipped` with the reason, and the
// group's gate runs more than once. `featureGateFirstRun` is the first run
// live; `featureGateSecondRun` is the second, with the first's red still in
// `check_runs` and `desktop_test` still running.

import type { CheckRun, CheckUnderway, DeclaredCheck, PlanGroup, PlanTask, StepDetail } from "@armada/protocol";
import type { JobFixture } from "@armada/screens/src/fixtures/fixture";

import { featureOnItsPlan } from "./job-detail-fixtures";

const AT = (minute: number) => `2026-10-07T04:${String(minute).padStart(2, "0")}:00.000Z`;

/** Fleet's own sentence for a Check the step touched no path of. */
const NOT_REACHED = "no changed file is under paths no workspace owns";

const CHECKS: DeclaredCheck[] = [
  { kind: "manifest_check", name: "build", run: "cargo build --workspace --locked", expect_exit_code: 0, when: ["crates/**"] },
  { kind: "manifest_check", name: "test", run: "cargo nextest run --workspace", expect_exit_code: 0, when: ["crates/**"] },
  { kind: "manifest_check", name: "typecheck", run: "pnpm typecheck", expect_exit_code: 0 },
  { kind: "manifest_check", name: "desktop_test", run: "pnpm exec vitest run", expect_exit_code: 0 },
  { kind: "manifest_check", name: "xtask_test", run: "cargo nextest run -p xtask", expect_exit_code: 0 },
];

const SKIPPED = ["build", "test"];

const TASKS: PlanTask[] = [
  { id: "T1", title: "Read the gate's rows", scope: ["crates/ipc/src/underway.rs", "packages/surfaces/jobs/src/plan-board.ts"], state: "done", group: "G1" },
  { id: "T2", title: "Draw a skipped Check", scope: ["packages/components/src/compositions/GroupBoundary/GroupBoundary.tsx"], state: "done", group: "G1" },
];

/** The first run, finished: skipped where nothing was touched, `desktop_test` red. */
const FIRST_RUN: CheckRun[] = CHECKS.map((check) => {
  const at = { attempt: 2, group: "G1", group_attempt: 1, name: check.name ?? "" };
  if (SKIPPED.includes(at.name)) return { ...at, outcome: "skipped", produced: NOT_REACHED };
  if (at.name === "desktop_test") return { ...at, outcome: "failed", expected: "`desktop_test` exits 0", produced: "it exited 1" };
  return { ...at, outcome: "passed" };
});

/** One Check as the gate has it: answered, started, or waiting. */
function underway(name: string, attempt: number, now: "skipped" | "passed" | "running" | "waiting"): CheckUnderway {
  if (now === "waiting") return { name };
  const started_at = AT(attempt === 2 ? 40 : 50);
  if (now === "running") return { name, started_at };
  const ran: CheckRun =
    now === "skipped" ? { attempt, name, outcome: "skipped", produced: NOT_REACHED } : { attempt, name, outcome: "passed" };
  return { name, started_at, ran, took_ms: now === "skipped" ? 0 : 4_000 };
}

function built(live: { state: PlanGroup["state"]; attempt: number; rows: CheckRun[]; run: "first" | "second" }): JobFixture {
  const base = featureOnItsPlan();
  if (base.watched.state !== "read") return base;
  const whole = base.watched.detail;
  const checking = {
    attempt: live.attempt,
    checks: CHECKS.map((check) => {
      const name = check.name ?? "";
      if (SKIPPED.includes(name)) return underway(name, live.attempt, "skipped");
      if (name === "typecheck") return underway(name, live.attempt, "passed");
      return underway(name, live.attempt, name === "desktop_test" ? "running" : "waiting");
    }),
  };
  const steps: StepDetail[] = whole.steps.map((step) => {
    if (step.step_id === "plan") {
      return { ...step, state: "advanced", attempts: [{ attempt: 1, outcome: "advanced", started_at: AT(0), ended_at: AT(1) }] };
    }
    if (step.step_id !== "implement") return step;
    return {
      ...step,
      state: "running",
      checks: CHECKS,
      check_runs: live.rows,
      checking,
      attempts: [
        { attempt: 1, outcome: "stopped", why: "run_ended", started_at: AT(1), ended_at: AT(30) },
        ...(live.run === "second"
          ? [{ attempt: 2, outcome: "retrying", why: "gate_failure", started_at: AT(31), ended_at: AT(45) } as const]
          : []),
        { attempt: live.attempt, outcome: "running", started_at: AT(live.run === "second" ? 46 : 31) },
      ],
      entered_at: AT(1),
      updated_at: AT(55),
    };
  });
  const group: PlanGroup = {
    id: "G1",
    tasks: ["T1", "T2"],
    state: live.state,
    started_at: AT(31),
    attempts:
      live.run === "second"
        ? [
            { attempt: 1, step_id: "implement", step_attempt: 2, started_at: AT(31), ended_at: AT(45), verdict: { attempt: 1, named: "failed", trigger: "gate_failure" } },
            { attempt: 2, step_id: "implement", step_attempt: live.attempt, started_at: AT(46) },
          ]
        : [{ attempt: 1, step_id: "implement", step_attempt: live.attempt, started_at: AT(31) }],
  };
  const job = {
    ...base.job,
    title: "Draw a Job's Checks as Fleet recorded them",
    status: "running",
    current_step_id: "implement",
    tasks: { done: 2, working: 0, open: 0, dropped: 0 },
  };
  const detail = {
    ...whole,
    job,
    steps,
    work_plan: {
      ...whole.work_plan!,
      approach: "Read the gate's rows as Fleet recorded them, then draw a skipped Check as skipped.",
      tasks: TASKS,
      groups: [group],
    },
  };
  return { ...base, job, watched: { ...base.watched, detail } };
}

/** The gate's first run is live: nothing is recorded yet, and the skipped Checks have answered. */
export function featureGateFirstRun(): JobFixture {
  return built({ state: "running", attempt: 2, rows: [], run: "first" });
}

/** The second run is live: the first's `desktop_test` red is recorded, and the second's is still running. */
export function featureGateSecondRun(): JobFixture {
  return built({ state: "retrying", attempt: 3, rows: FIRST_RUN, run: "second" });
}
