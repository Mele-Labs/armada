// A feature Job mid-Implement with a ten-task plan, the shape of real Job 14: "Wire the Cockpit's Job
// detail panel and Job call cards to real Fleet data". Three stages, each a moment a Commander is
// drawn against: T7's Drone working, T7's Drone stuck at the turn cap, and Group 3 red with its
// retries spent. Mock data only, built on `featureRunning`'s Job. The walks `job-commander-*` play them.

import type { CheckRun, JobDetail, PlanGroup, PlanTask, StepDetail } from "@armada/protocol";
import type { JobFixture } from "@armada/screens/src/fixtures/fixture";

import { featureRunning } from "@armada/screens/src/fixtures/jobs/feature-running";

export type CommanderStage = "working" | "stuck" | "red";

const AT = (minute: number) => `2026-10-11T13:${String(minute).padStart(2, "0")}:00.000Z`;

export const COMMANDER_REQUEST = "Wire the Cockpit's Job detail panel and Job call cards to real Fleet data";

/** The files a Drone's ask touches: the wire and main first, then what the Cockpit draws. */
const ASK_SCOPE = ["crates/ipc/src/asks.rs", "apps/desktop/src/main/asks.ts", "packages/surfaces/jobs/src/now-real.ts", "apps/desktop/src/renderer/src/cockpit/CallCard.tsx"];

const TASKS: PlanTask[] = [
  { id: "T1", title: "Serve each Job's Now rows from Fleet", scope: ["crates/ipc/src/now.rs"], state: "done", group: "G1", concurrent_with: ["T2"] },
  { id: "T2", title: "Read a Job's request and origin onto its call", scope: ["crates/ipc/src/calls.rs"], state: "done", group: "G1", concurrent_with: ["T1"] },
  { id: "T3", title: "Carry the asking Drone's words to the Cockpit", scope: ["apps/desktop/src/main/questions.ts"], state: "done", group: "G2" },
  { id: "T4", title: "Draw the Job panel from Fleet's plan groups", scope: ["packages/surfaces/jobs/src/plan-board.ts"], state: "done", group: "G2" },
  { id: "T5", title: "Show a task's Drone and its turns in the panel", scope: ["packages/surfaces/jobs/src/job-drones.tsx"], state: "done", group: "G2" },
  { id: "T6", title: "Answer a Plan question from the call card", scope: ["apps/desktop/src/renderer/src/cockpit/answers.ts"], state: "done", group: "G3" },
  { id: "T7", title: "Raise a Drone's permission ask into the Cockpit", scope: ASK_SCOPE, state: "working", group: "G3" },
  { id: "T8", title: "Wire Retry and Skip on a failed Check row", scope: ["apps/desktop/src/renderer/src/cockpit/CallCard.tsx"], state: "open", group: "G3" },
  { id: "T9", title: "Open a Drone's transcript from the call card", scope: ["apps/desktop/src/renderer/src/cockpit/CallCard.tsx"], state: "open", group: "G3" },
  { id: "T10", title: "Replace the Cockpit's mock fixtures with the wired reads", scope: ["apps/desktop/src/renderer/src/mock/scenarios"], state: "open", group: "G4" },
];

const passed = (id: string, tasks: string[], from: number, to: number, commit: string): PlanGroup => ({
  id,
  tasks,
  state: "passed",
  started_at: AT(from),
  ended_at: AT(to),
  attempts: [{ attempt: 1, step_id: "implement", step_attempt: 1, started_at: AT(from), ended_at: AT(to), verdict: { attempt: 1, named: "passed" }, commit }],
});

const GROUPS = (stage: CommanderStage): PlanGroup[] => [
  passed("G1", ["T1", "T2"], 1, 9, "4c1b9d2"),
  passed("G2", ["T3", "T4", "T5"], 10, 24, "7a2f0c5"),
  stage === "red"
    ? {
        id: "G3",
        tasks: ["T6", "T7", "T8", "T9"],
        state: "failed",
        started_at: AT(25),
        ended_at: AT(52),
        attempts: [1, 2, 3].map((run) => ({
          attempt: run,
          step_id: "implement",
          step_attempt: 1,
          started_at: AT(25 + run * 8),
          ended_at: AT(31 + run * 8),
          verdict: { attempt: run, named: "failed", trigger: "gate_failure" },
        })),
      }
    : {
        id: "G3",
        tasks: ["T6", "T7", "T8", "T9"],
        state: "running",
        started_at: AT(25),
        attempts: [{ attempt: 1, step_id: "implement", step_attempt: 1, started_at: AT(25) }],
      },
  { id: "G4", tasks: ["T10"], state: "pending" },
];

/** Where each task stands at each stage. */
function tasksAt(stage: CommanderStage): PlanTask[] {
  return TASKS.map((task) => {
    if (stage === "working") return task;
    if (stage === "stuck") return task;
    // Red: G3's tasks ran to the end and its Checks held it back.
    if (task.id === "T7" || task.id === "T8") return { ...task, state: "done" };
    if (task.id === "T9") return { ...task, state: "failed", failed_reason: "G3's Checks were still red on run 3, the last its retries allow: desktop_test" };
    return task;
  });
}

const countsOf = (tasks: readonly PlanTask[]) => {
  const count = (state: string) => tasks.filter((task) => task.state === state).length;
  return {
    done: count("done"),
    working: count("working"),
    open: count("open"),
    dropped: count("dropped"),
    ...(count("failed") === 0 ? {} : { failed: count("failed") }),
  };
};

/** The Checks G3's three runs ran: `typecheck` green, `desktop_test` red every time. */
function redRuns(): CheckRun[] {
  return [1, 2, 3].flatMap((run) => [
    { attempt: 1, group: "G3", group_attempt: run, name: "typecheck", outcome: "passed" },
    { attempt: 1, group: "G3", group_attempt: run, name: "desktop_test", outcome: "failed", expected: "exits 0", produced: "exited 1, 3 of 2210 failed" },
  ]);
}

export function commanderJob(stage: CommanderStage = "working"): JobFixture {
  const base = featureRunning();
  if (base.watched.state !== "read") return base;
  const whole = base.watched.detail;
  const plan = whole.work_plan!;
  const tasks = tasksAt(stage);
  const job = { ...base.job, title: COMMANDER_REQUEST, status: "running", tasks: countsOf(tasks) };
  const steps: StepDetail[] = whole.steps.map((step) =>
    step.step_id !== "implement" || stage !== "red" ? step : { ...step, check_runs: [...(step.check_runs ?? []), ...redRuns()] },
  );
  const detail: JobDetail = { ...whole, job, steps, work_plan: { ...plan, approach: "Serve what the Cockpit draws from Fleet, then wire each act.", tasks, groups: GROUPS(stage) } };
  return { ...base, job, watched: { ...base.watched, detail }, name: `running: Job 14's Implement, the plan at ${stage}` };
}
