// A feature Job past its gate and mid-Implement, for the running-Job canvas
// (prototype, 4 Oct 2026): `featureRunInGroups`' plan, Fleet's groups as it
// serves them since 23.4, the first two passed with their commits and the
// third running its first try — so the canvas draws where the Job is, and
// Groups filled in with the plan's own.

import type { PlanGroup, StepDetail } from "@armada/protocol";
import type { JobFixture } from "@armada/screens/src/fixtures/fixture";

import { featureRunInGroups } from "./job-groups-fixture";

export function featureRunning(): JobFixture {
  const base = featureRunInGroups();
  if (base.watched.state !== "read") return base;
  const whole = base.watched.detail;
  const plan = whole.work_plan!;
  // G3 on its first run, nothing back yet; T4 being worked.
  const groups: PlanGroup[] = (plan.groups ?? []).map((group) => {
    if (group.id !== "G3") return group;
    const { ended_at: _ended, ...rest } = group;
    return { ...rest, state: "running", attempts: (group.attempts ?? []).slice(0, 1).map(({ verdict: _v, ended_at: _e, ...one }) => one) };
  });
  const tasks = plan.tasks.map((task) => {
    if (task.id !== "T4") return task;
    const { failed_reason: _reason, ...rest } = task;
    return { ...rest, state: "working" as const };
  });
  const steps: StepDetail[] = whole.steps.map((step) => {
    if (step.step_id !== "implement") return step;
    const { last_verdict: _verdict, ...rest } = step;
    return {
      ...rest,
      // `feature.json` declares a Drone per task here, as Fleet serves it since 23.1.
      drone_per_task: true,
      state: "running",
      attempts: step.attempts.slice(0, 1).map(({ why: _w, ended_at: _e, ...one }) => ({ ...one, outcome: "running" })),
      check_runs: (step.check_runs ?? []).filter((run) => run.group !== "G3"),
    };
  });
  const job = {
    ...base.job,
    status: "running",
    tasks: { done: 3, working: 1, open: 0, dropped: 0, failed: 0 },
  };
  return {
    ...base,
    // The repository's branches, so where it lands is picked from them.
    branches: {
      branches: [
        { name: "main", base: false },
        { name: "release/2026-10", base: false },
        { name: "armada/1-draw-the-plans-groups", base: false },
      ],
    },
    name: "running — a feature Job mid-Implement, its canvas marked with where it is",
    job,
    // Approved before it ran, as Fleet stamps it since 23.8.
    watched: {
      ...base.watched,
      detail: { ...whole, job, steps, work_plan: { ...plan, tasks, groups }, approved_at: "2026-10-02T13:58:00.000Z" },
    },
  };
}
