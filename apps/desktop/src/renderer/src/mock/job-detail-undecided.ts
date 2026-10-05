// Job 3 at the Judge's silence on its plan, 5 Oct 2026: the plan step's Check
// passed, the Judge call died ("Reached max turns (8)") and the step stopped on
// `gate_undecided`. Apart from `job-detail-refusal.ts`, which is the same Job at
// a Judge that did answer.

import type { StepDetail } from "@armada/protocol";
import type { JobFixture } from "@armada/screens/src/fixtures/fixture";

import { featureOnItsPlan } from "./job-detail-fixtures";

/** Fleet's own sentence for why the Judge gave no verdict, as it crosses the wire. */
const UNDECIDED = "Reached max turns (8)";

/**
 * The plan step stopped, the Job escalated, and `stuck` offering both
 * recourses the trigger now admits: asking the Judge again and accepting the
 * step. `redirect_drone` is there because the Drone is still standing.
 */
export function featureUndecided(): JobFixture {
  const base = featureOnItsPlan();
  if (base.watched.state !== "read") return base;
  const whole = base.watched.detail;
  const stoppedAt = "2026-10-05T14:10:00.000Z";
  const steps = whole.steps.map((one): StepDetail =>
    one.step_id !== "plan"
      ? one
      : {
          ...one,
          state: "stopped",
          check_runs: [{ attempt: 1, name: "plan_recorded", outcome: "passed" }],
          // Asked again once: attempt 2 re-ran no Check, so `check_runs` names attempt 1 alone.
          attempts: [
            { attempt: 1, outcome: "stopped", why: "gate_undecided", started_at: one.entered_at, ended_at: stoppedAt },
            { attempt: 2, outcome: "stopped", why: "gate_undecided", started_at: stoppedAt },
          ],
          verdicts: [
            { attempt: 1, named: "failed", trigger: "gate_undecided" },
            { attempt: 2, named: "failed", trigger: "gate_undecided" },
          ],
          last_verdict: { attempt: 2, named: "failed", trigger: "gate_undecided" },
          updated_at: stoppedAt,
        },
  );
  const job = { ...base.job, status: "escalated", reason: { named: "gate_undecided" } };
  const detail = {
    ...whole,
    job,
    steps,
    stuck: {
      stopped_by: "gate_undecided",
      undecided: UNDECIDED,
      step_id: "plan",
      recourse: ["rerun_gate", "override_verdict", "redirect_drone", "redispatch_job"],
      worktree_on_disk: true,
      drone_unheard: false,
      refused: [],
      refusals: 0,
    },
  };
  return { ...base, job, watched: { ...base.watched, detail } };
}
