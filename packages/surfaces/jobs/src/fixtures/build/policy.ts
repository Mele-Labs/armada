// `awaiting_review` — held by the repository's own review-gate policy, on a
// step that was sent back once. #1683.
//
// **Three runs of one step.** Regression check defers to the repository here
// (`manifest_rule:review_gate`). The first held for a person on a Fleet older
// than 21.9, which recorded nothing about what the policy resolved to, so it
// carries no `resolved` and the Record says nothing about it. The second's
// Checks failed, so it kept what the rule said and never reached the gate
// where the rule decides: `decided: false`. The third held, and carries both
// policies spelled the way `armada.yml` writes them — the value
// `crates/ipc/src/tests/absent.rs` round-trips — with `decided: true`.
//
// **It carries the Job's history**, because Bridge reads it whenever a Job is
// open, so the Record's run rows are the log's own step moves rather than rows
// derived from `attempts`.

import type { Recorded, StepAttempt, StepDetail } from "@armada/protocol";
import type { JobFixture } from "@armada/screens/src/fixtures/fixture";
import { workflow } from "@armada/screens/src/fixtures/build/base";
import { review } from "./review";

const GATED = "manifest_rule:review_gate";

/** The first run's hold, the send-back, the failed Checks, the retry and the hold. */
const HELD_FIRST = "2026-09-10T14:29:40Z";
const SENT_BACK = "2026-09-10T14:29:50Z";
const READMITTED = "2026-09-10T14:29:55Z";
const CHECKS_FAILED = "2026-09-10T14:30:10Z";
const RETRIED = "2026-09-10T14:30:11Z";
const HELD_AGAIN = "2026-09-10T14:30:40Z";

/** What both policies resolved to, as `armada.yml` writes them. */
const RESOLVED = { auto_merge: "checks-pass", review_gate: "human_always" };

/** Every run. Only the last two ran on a Fleet that records the policies. */
const RUNS: StepAttempt[] = [
  { attempt: 1, outcome: "awaiting_human", started_at: "2026-09-10T14:22:18Z", ended_at: HELD_FIRST },
  {
    attempt: 2,
    outcome: "retrying",
    why: "gate_failure",
    started_at: READMITTED,
    ended_at: CHECKS_FAILED,
    resolved: { ...RESOLVED, decided: false },
  },
  {
    attempt: 3,
    outcome: "awaiting_human",
    started_at: RETRIED,
    ended_at: HELD_AGAIN,
    resolved: { ...RESOLVED, decided: true },
  },
];

/** Every step's gate as the workflow declares it, with Regression check deferring to the repository. */
function gated(step: StepDetail): StepDetail {
  const declared = workflow().steps.find((one) => one.step_id === step.step_id)?.advance_gate;
  if (step.step_id === "regression_verify") {
    return {
      ...step,
      advance_gate: GATED,
      attempts: RUNS,
      // The second run's Checks failed, so the Judge never read it.
      check_runs: [
        ...step.check_runs,
        ...step.check_runs.map((run) => ({ ...run, attempt: 2, outcome: "failed" })),
        ...step.check_runs.map((run) => ({ ...run, attempt: 3 })),
      ],
      judged: [...step.judged, ...step.judged.map((one) => ({ ...one, attempt: 3 }))],
      updated_at: HELD_AGAIN,
    };
  }
  return declared === undefined ? step : { ...step, advance_gate: declared };
}

/** One recorded move. `seq` is the log's, in order. */
function move(seq: number, status: string, moved: Recorded["moved"], actor: string, at: string): Recorded {
  return { seq, status, moved, actor, at };
}

/** A step's own two moves, in and out of `running`. */
function ran(seq: number, stepId: string, from: string, to: string, at: string): Recorded {
  return move(seq, "running", { kind: "step", step_id: stepId, from, to }, "fleet", at);
}

/** `GET /jobs/:job_id/events`, oldest first, for the runs this Job has had. */
function history(): Recorded[] {
  return [
    move(1, "awaiting_approval", { kind: "status", to: "queued" }, "human", "2026-09-10T14:10:40Z"),
    move(2, "queued", { kind: "status", to: "running" }, "fleet", "2026-09-10T14:11:02Z"),
    ran(3, "repro", "not_started", "running", "2026-09-10T14:11:15Z"),
    ran(4, "repro", "running", "advanced", "2026-09-10T14:12:27Z"),
    ran(5, "root_cause", "not_started", "running", "2026-09-10T14:12:27Z"),
    ran(6, "root_cause", "running", "advanced", "2026-09-10T14:16:07Z"),
    ran(7, "fix", "not_started", "running", "2026-09-10T14:16:07Z"),
    ran(8, "fix", "running", "advanced", "2026-09-10T14:22:18Z"),
    ran(9, "regression_verify", "not_started", "running", "2026-09-10T14:22:18Z"),
    ran(10, "regression_verify", "running", "awaiting_human", HELD_FIRST),
    move(11, "running", { kind: "status", to: "awaiting_review" }, "fleet", HELD_FIRST),
    // A person sent it back: the Job re-queues and a fresh Drone takes the step.
    move(12, "awaiting_review", { kind: "status", to: "queued" }, "human", SENT_BACK),
    move(13, "queued", { kind: "status", to: "running" }, "fleet", READMITTED),
    ran(14, "regression_verify", "awaiting_human", "running", READMITTED),
    // Its Checks failed, inside the retry budget.
    move(
      15,
      "running",
      { kind: "step", step_id: "regression_verify", from: "running", to: "retrying", why: "gate_failure" },
      "fleet",
      CHECKS_FAILED,
    ),
    ran(16, "regression_verify", "retrying", "running", RETRIED),
    ran(17, "regression_verify", "running", "awaiting_human", HELD_AGAIN),
    move(18, "running", { kind: "status", to: "awaiting_review" }, "fleet", HELD_AGAIN),
  ];
}

export function reviewHeldByPolicy(): JobFixture {
  const base = review();
  const watched = base.watched;
  if (watched.state !== "read") throw new Error("review() reads its Job whole");
  const steps = watched.detail.steps.map(gated);
  return {
    ...base,
    name: "awaiting_review — held because the repository says a person answers",
    watched: { ...watched, detail: { ...watched.detail, steps } },
    workflows: [
      {
        ...workflow(),
        steps: workflow().steps.map((one) =>
          one.step_id === "regression_verify" ? { ...one, advance_gate: GATED } : one,
        ),
      },
    ],
    history: { state: "read", jobId: base.job.id, moves: history() },
  };
}
