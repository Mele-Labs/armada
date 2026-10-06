// Five statuses a person or a queue is holding this Job at, light depth.
//
// **Nothing here is full-depth**, per the roster: no turns, no journal, no
// Check outputs — just enough of `JobDetail` for `renderFor` to land on the
// right render and for the panel to draw without a step to say anything about.

import type { CheckRun, DeclaredCheck, JobResources, StepDetail } from "@armada/protocol";
import type { JobFixture } from "../fixture";
import {
  advancedStep,
  BUILD_CHECK,
  consumersStep,
  detail,
  freshStep,
  holdsRead,
  job,
  JOB_ID,
  landStep,
  manifest,
  NEXTEST_CHECK,
  NO_JOURNALLED,
  NO_OBSERVED,
  NOW,
  reproStep,
  rootCauseStep,
  spend,
  stuck,
  watchedRead,
  workflow,
} from "./base";
import { foldedReads } from "./folded";

/** No worktree yet — every step still ahead. Used by both gate statuses. */
function beforeDispatch(): { steps: StepDetail[]; resources: JobResources } {
  return {
    steps: [
      freshStep("repro", "Reproduction", 1),
      freshStep("root_cause", "Root cause", 2),
      freshStep("fix", "Fix", 3, [BUILD_CHECK]),
      freshStep("regression_verify", "Regression check", 4, [NEXTEST_CHECK]),
      consumersStep(),
      landStep(),
    ],
    resources: { job_id: JOB_ID, read_at: "2026-09-10T14:12:00.000Z", held: "none", processes: [] },
  };
}

export function queued(): JobFixture {
  const before = beforeDispatch();
  const theJob = job("queued", {
    current_step_id: "repro",
    queued_reason: "waiting_on_resources",
    branch: undefined,
    assigned_drone: undefined,
  });
  const whole = detail(theJob, before.steps, { branch: undefined, spend: spend({ cost_micros: 0, turns: 0, ran_ms: 0, drones: 0 }) });
  return {
    name: "queued — approved, waiting on a free drone",
    job: theJob,
    watched: watchedRead(whole),
    workflows: [workflow()],
    manifests: [manifest()],
    observed: NO_OBSERVED,
    journalled: NO_JOURNALLED,
    resources: holdsRead(before.resources),
    recorded: foldedReads(),
    checkOutputs: {},
    frames: {},
    now: NOW,
  };
}

export function awaitingApproval(): JobFixture {
  const before = beforeDispatch();
  const theJob = job("awaiting_approval", {
    current_step_id: "repro",
    branch: undefined,
    assigned_drone: undefined,
  });
  const whole = detail(theJob, before.steps, { branch: undefined, spend: spend({ cost_micros: 0, turns: 0, ran_ms: 0, drones: 0 }) });
  return {
    name: "awaiting_approval — a person must approve the dispatch",
    job: theJob,
    watched: watchedRead(whole),
    workflows: [workflow()],
    manifests: [manifest()],
    observed: NO_OBSERVED,
    journalled: NO_JOURNALLED,
    resources: holdsRead(before.resources),
    recorded: foldedReads(),
    checkOutputs: {},
    frames: {},
    now: NOW,
  };
}

function regressionSpentRetries(): StepDetail {
  return {
    ...freshStep("regression_verify", "Regression check", 4, [NEXTEST_CHECK]),
    state: "stopped",
    check_runs: [
      { attempt: 1, name: "cargo_nextest", outcome: "failed", produced: "exit 101" },
      { attempt: 2, name: "cargo_nextest", outcome: "failed", produced: "exit 101" },
    ],
    attempts: [
      { attempt: 1, outcome: "retrying", why: "gate_failure", started_at: "2026-09-10T14:22:18Z", ended_at: "2026-09-10T14:24:00Z" },
      { attempt: 2, outcome: "stopped", why: "gate_failure", started_at: "2026-09-10T14:24:00Z", ended_at: "2026-09-10T14:25:40Z" },
    ],
    verdicts: [{ attempt: 2, named: "failed", trigger: "gate_failure" }],
    last_verdict: { attempt: 2, named: "failed", trigger: "gate_failure" },
    entered_at: "2026-09-10T14:22:18Z",
    updated_at: "2026-09-10T14:25:40Z",
  };
}

export function awaitingRepair(): JobFixture {
  const theJob = job("awaiting_repair", { current_step_id: "regression_verify", assigned_drone: undefined });
  const steps = [
    reproStep(),
    rootCauseStep(),
    advancedStep("fix", "Fix", 3, [BUILD_CHECK]),
    regressionSpentRetries(),
    consumersStep(),
    landStep(),
  ];
  const whole = detail(theJob, steps, {
    stuck: stuck({
      step_id: "regression_verify",
      recourse: ["rerun_checks", "restart_step", "redispatch_job"],
      worktree_on_disk: true,
      drone_unheard: false,
    }),
  });
  return {
    name: "awaiting_repair — the retry budget is spent and the work is unfinished",
    job: theJob,
    watched: watchedRead(whole),
    workflows: [workflow()],
    manifests: [manifest()],
    observed: NO_OBSERVED,
    journalled: NO_JOURNALLED,
    resources: holdsRead({
      job_id: JOB_ID,
      read_at: "2026-09-10T14:26:00.000Z",
      held: "none",
      processes: [],
      worktree: { path: ".armada/worktrees/77-split-the-settings-reducer", branch: "fix/settings-split-selectors", bytes: 1_310_720_000, measured_at: "2026-09-10T14:25:40.000Z" },
    }),
    recorded: foldedReads(),
    checkOutputs: {},
    frames: {},
    now: NOW,
  };
}

/** The four Checks the owner's Job 3 ran again on 4 Oct 2026, by their Manifest names. */
const RERUN: [name: string, run: string][] = [
  ["bridge_build", "pnpm -C apps/desktop build"],
  ["storybook", "pnpm -C packages/components build-storybook"],
  ["typecheck", "pnpm typecheck"],
  ["desktop_test", "pnpm --dir apps/desktop exec vitest run"],
];

/**
 * `awaitingRepair` after a person pressed Run Checks again, as the owner's Job 3
 * served it on 4 Oct 2026: the status and the step's `stopped` unchanged, Fleet
 * offering nothing while the run is out, and the gate's live set on the stopped
 * step — three Checks passed this time and `desktop_test` running.
 *
 * **Not in `FIXTURES`**: it is a moment of `awaitingRepair`, not a state a Job
 * can be listed in.
 */
export function awaitingRepairChecksAgain(): JobFixture {
  const base = awaitingRepair();
  if (base.watched.state !== "read") return base;
  const whole = base.watched.detail;
  const checks: DeclaredCheck[] = RERUN.map(([name, run]) => ({ kind: "manifest_check", name, run, expect_exit_code: 0 }));
  const ran = (attempt: number, name: string): CheckRun =>
    name === "desktop_test" ? { attempt, name, outcome: "failed", produced: "exit 1" } : { attempt, name, outcome: "passed" };
  const passed = (name: string, started_at: string, took_ms: number) => ({
    name,
    started_at,
    took_ms,
    ran: { attempt: 2, name, outcome: "passed" },
  });
  const steps = whole.steps.map(
    (step): StepDetail =>
      step.step_id !== "regression_verify"
        ? step
        : {
            ...step,
            checks,
            check_runs: [1, 2].flatMap((attempt) => RERUN.map(([name]) => ran(attempt, name))),
            checking: {
              attempt: 2,
              checks: [
                passed("bridge_build", "2026-09-10T14:28:00Z", 41_000),
                passed("storybook", "2026-09-10T14:28:41Z", 52_000),
                passed("typecheck", "2026-09-10T14:29:33Z", 37_000),
                { name: "desktop_test", started_at: "2026-09-10T14:30:10Z" },
              ],
            },
          },
  );
  return {
    ...base,
    name: "awaiting_repair — its Checks running again",
    watched: watchedRead({ ...whole, steps, stuck: { ...whole.stuck!, recourse: [] } }),
  };
}

export function awaitingAttestation(): JobFixture {
  const theJob = job("awaiting_attestation", {
    current_step_id: "land",
    reason: { criteria_owed: ["c2"] },
  });
  const steps = [
    reproStep(),
    rootCauseStep(),
    advancedStep("fix", "Fix", 3, [BUILD_CHECK]),
    advancedStep("regression_verify", "Regression check", 4, [NEXTEST_CHECK]),
    advancedStep("consumers", "Check the consumers still compile", 5, [BUILD_CHECK]),
    advancedStep("land", "Land", 6),
  ];
  const whole = detail(theJob, steps);
  return {
    name: "awaiting_attestation — the work landed, a criterion needs a person's action outside Armada",
    job: theJob,
    watched: watchedRead(whole),
    workflows: [workflow()],
    manifests: [manifest()],
    observed: NO_OBSERVED,
    journalled: NO_JOURNALLED,
    resources: holdsRead({
      job_id: JOB_ID,
      read_at: "2026-09-10T14:31:00.000Z",
      held: "none",
      processes: [],
      worktree: { path: ".armada/worktrees/77-split-the-settings-reducer", branch: "fix/settings-split-selectors", bytes: 1_476_395_008, measured_at: "2026-09-10T14:30:40.000Z" },
    }),
    recorded: foldedReads(),
    checkOutputs: {},
    frames: {},
    now: NOW,
  };
}

export function piloted(): JobFixture {
  const theJob = job("piloted", {
    current_step_id: "fix",
    reason: { named: "take_over" },
    assigned_drone: undefined,
  });
  const steps = [
    reproStep(),
    rootCauseStep(),
    { ...freshStep("fix", "Fix", 3, [BUILD_CHECK]), state: "running", attempts: [{ attempt: 1, outcome: "running", started_at: "2026-09-10T14:16:07Z" }] },
    freshStep("regression_verify", "Regression check", 4, [NEXTEST_CHECK]),
    consumersStep(),
    landStep(),
  ];
  const whole = detail(theJob, steps);
  return {
    name: "piloted — a person is working it now, and the drone is gone",
    job: theJob,
    watched: watchedRead(whole),
    workflows: [workflow()],
    manifests: [manifest()],
    observed: NO_OBSERVED,
    journalled: NO_JOURNALLED,
    resources: holdsRead({
      job_id: JOB_ID,
      read_at: "2026-09-10T14:31:00.000Z",
      held: "none",
      processes: [],
      worktree: { path: ".armada/worktrees/77-split-the-settings-reducer", branch: "fix/settings-split-selectors", bytes: 1_310_720_000, measured_at: "2026-09-10T14:30:40.000Z" },
    }),
    recorded: foldedReads(),
    checkOutputs: {},
    frames: {},
    now: NOW,
  };
}
