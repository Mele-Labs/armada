// A running feature Job staged for the gate stages (prototype, 5 Oct 2026):
// every kind of gate and every state in one frame. **Staged, not a moment** —
// a real Job is at one step, and here three steps stand at their gates at once
// so the owner can read each face beside the others.
//
//   plan       its Checks and Judge passed
//   implement  attempt 2 of 3: Checks running, one failed, a live output line
//   handoff    the Judge passed, You held

import type { CheckRun, Judged, StepDetail } from "@armada/protocol";
import type { JobFixture } from "@armada/screens/src/fixtures/fixture";

import { featureRunning } from "./feature-running";

const ago = (minutes: number): string => new Date(Date.now() - minutes * 60_000).toISOString();

const namesOf = (step: StepDetail): string[] => (step.checks ?? []).map((check) => check.name ?? check.kind);

export function featureGates(): JobFixture {
  const base = featureRunning();
  if (base.watched.state !== "read") return base;
  const whole = base.watched.detail;
  const run = (name: string, attempt: number, outcome: string, produced?: string): CheckRun => ({
    attempt,
    name,
    outcome,
    ...(produced === undefined ? {} : { produced }),
  });
  const judged = (attempt: number, member: number): Judged => ({ attempt, criterion_id: "C1", member, verdict: "met" });
  const steps: StepDetail[] = whole.steps.map((step) => {
    const names = namesOf(step);
    switch (step.step_id) {
      case "plan":
        return {
          ...step,
          check_runs: names.map((name) => run(name, 1, "passed")),
          judged: [judged(1, 0)],
        };
      case "implement":
        return {
          ...step,
          pass: { number: 2, of: 3 },
          verdict_routing_target: "implement",
          attempts: [
            { attempt: 1, outcome: "refused", started_at: ago(52), ended_at: ago(30) },
            { attempt: 2, outcome: "running", started_at: ago(4) },
          ],
          check_runs: [
            run("build", 2, "passed"),
            run("typecheck", 2, "failed"),
            run("test", 2, "running", "test screens::approval_life::marks_a_gate ... ok"),
          ],
        };
      case "handoff":
        return {
          ...step,
          state: "awaiting_human",
          attempts: [{ attempt: 1, outcome: "awaiting_human", started_at: ago(23) }],
          judged: [judged(1, 0)],
        };
      default:
        return step;
    }
  });
  return {
    ...base,
    name: "gates — a feature Job with a gate stage in every state",
    watched: { ...base.watched, detail: { ...whole, steps } },
  };
}
