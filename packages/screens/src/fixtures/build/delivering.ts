// `awaiting_review` at the delivering step — the branch has gone out, a pull
// request is open, and `Decide`'s fourth answer is what a person is looking
// at.
//
// **`land` is `human_always` here and nowhere else in this roster.** The
// shared `workflow()` in `base.ts` leaves it ungated, because every other
// fixture's Job holds at `regression_verify` or earlier and never opens
// `land`'s own facts — changing the shared declaration would be data nobody
// reads moving for nobody's benefit. This step is written by hand instead,
// the way `gating.ts`'s two-Check step is, so nothing else in the roster
// changes shape.
//
// **Two moments at that gate**, since the lead counts the whole Job's evidence:
// a clean one, and `reviewAfterAnOverrule` — the only Job on any roster with a
// retried Check and an overruled criterion behind a branch that is out.
//
// **`WithNoPullRequest` is not a fixture here.** `review()` already is that
// state — `awaiting_review`, mid-workflow, `whole.delivery` absent — and the
// same absence that keeps its own gate un-mergeable is what keeps `land`
// undrawn there. A second builder for the same absence would be the same
// state told twice.

import type { JobFixture } from "../fixture";
import type { Remark, Remarks, StepDetail } from "@armada/protocol";
import {
  advancedStep,
  BUILD_CHECK,
  buildRun,
  detail,
  diffRead,
  droneEnded,
  evidenceRead,
  foldedReads,
  holdsRead,
  instructed,
  job,
  journalledWatching,
  JOB_ID,
  manifest,
  NEXTEST_CHECK,
  note,
  NOW,
  observedEnded,
  reproStep,
  resources,
  rootCauseStep,
  said,
  verdict,
  watchedRead,
  workflow,
} from "./base";

/** The pull request address this Job's branch went out on. Same style as `terminal.ts`'s. */
const PULL_REQUEST = "https://git.example/armada/settings/pull/512";

/**
 * What people wrote on this Job's own pull request. `Decide.tsx` draws this
 * only where `whole.delivery.pull_request` is present, so the two have to
 * name the same address — kept local to this file rather than added to
 * `base.ts`, which every other fixture pays the size of.
 */
function remarksRead(pullRequest: string, remarks: Remark[]): Remarks {
  return { state: "read", jobId: JOB_ID, review: { job_id: JOB_ID, pull_request: pullRequest, remarks } };
}

/** `regression_verify`, advanced — the gate this roster's `review` fixture holds at, now behind it. */
function regressionAdvanced(): StepDetail {
  return {
    ...advancedStep("regression_verify", "Regression check", 4, [NEXTEST_CHECK]),
    judge_checks: [{ criteria: 2, gaming_check: true }],
    judged: [
      { attempt: 1, criterion_id: "c1", verdict: "met" },
      { attempt: 1, criterion_id: "c2", verdict: "met" },
    ],
  };
}

/**
 * `land` — pushed the branch and opened the pull request, then stopped for
 * the delivering gate. No Check and no Judge: `docs/concepts/job.md`'s
 * delivering step verifies nothing of its own, and `phases.tsx` draws no tier
 * for either — the note under the strip is what says the step gates on
 * nothing, the way `PHASES_DELIVERED` in the retired story said it by hand.
 */
function landAtTheGate(): StepDetail {
  return {
    step_id: "land",
    label: "Land",
    ordinal: 6,
    state: "awaiting_human",
    checks: [],
    check_runs: [],
    judge_checks: [],
    judged: [],
    flagged: [],
    overridden: false,
    advance_gate: "human_always",
    attempts: [{ attempt: 1, outcome: "awaiting_human", started_at: "2026-09-10T14:26:56Z" }],
    verdicts: [],
    entered_at: "2026-09-10T14:26:56Z",
    updated_at: "2026-09-10T14:27:12Z",
  };
}

const FILES = [
  { path: "packages/settings/src/selectors.ts", change: "modified" },
  { path: "packages/settings/src/reducer.ts", change: "modified" },
  { path: "packages/settings/src/index.ts", change: "added" },
  { path: "packages/settings/test/useColumnSelectors.test.ts", change: "added" },
];

/**
 * What three reviewers wrote on the pull request, oldest first. Two are
 * change requests and the third is an unrelated question; the fourth is
 * already handed to a drone and cannot be picked again — `ReviewComments`'
 * own distinction, off `Remark.taken_up`.
 */
const REMARKS: Remark[] = [
  {
    id: "IC_kwDOgate0",
    by: "a-reviewer",
    at: "2026-09-09 08:58",
    said:
      "The test file has no case for an **empty** column list, which is the one the board " +
      "draws differently:\n\n- `columns: []`\n- one hidden column",
    taken_up: true,
  },
  {
    id: "IC_kwDOgate1",
    by: "a-reviewer",
    at: "2026-09-09 09:12",
    said: "selectColumnOrder is memoised on the whole settings slice again in the new module. That is the bug this job was dispatched for, moved rather than fixed.",
    taken_up: false,
  },
  {
    id: "IC_kwDOgate2",
    by: "a-reviewer",
    at: "2026-09-09 09:14",
    said: "index.ts re-exports the internal selectors as well as the public ones. Nothing outside the package should be able to reach selectVisibleColumnsRaw.",
    taken_up: false,
  },
  {
    id: "IC_kwDOgate3",
    by: "somebody-else",
    at: "2026-09-09 09:40",
    said: "Unrelated to this change, but is the density setting still read anywhere? I could not find a consumer.",
    taken_up: false,
  },
];

export function reviewAtDelivery(): JobFixture {
  const theJob = job("awaiting_review", { current_step_id: "land" });
  const steps = [
    reproStep(),
    rootCauseStep(),
    advancedStep("fix", "Fix", 3, [BUILD_CHECK]),
    regressionAdvanced(),
    advancedStep("consumers", "Check the consumers still compile", 5, [BUILD_CHECK]),
    landAtTheGate(),
  ];
  const whole = detail(theJob, steps, {
    delivery: { commit: "9f4e21b", pushed: "origin/fix/settings-split-selectors", pull_request: PULL_REQUEST },
  });

  const rows = [
    instructed("land", "2026-09-10T14:26:56Z", 6, "the branch is pushed and a pull request is open", "Land"),
    said("land", "2026-09-10T14:27:05Z", "Pushing the branch and opening a pull request against main."),
    droneEnded("land", "2026-09-10T14:27:12Z", 3, 60_000),
  ];

  return {
    name: "awaiting_review — the branch is pushed, a pull request is open, and the fourth answer appears",
    job: theJob,
    watched: watchedRead(whole),
    workflows: [workflow()],
    manifests: [manifest()],
    observed: observedEnded(rows, "drone_ended"),
    journalled: journalledWatching([
      note("2026-09-10T14:11:02Z", "A worktree was cut for this job."),
      note("2026-09-10T14:27:03Z", "Pull request opened against main.", { step: "land" }),
      note("2026-09-10T14:27:12Z", "Step complete. The workflow asks for a person here.", { step: "land" }),
    ]),
    resources: holdsRead(resources("none", { processes: [], wrote_last_at: "2026-09-10T14:27:12.000Z" })),
    recorded: foldedReads({
      evidence: evidenceRead([
        {
          step_id: "regression_verify",
          evidence_type: "test_suite_run",
          claimed: "2034 of 2034 tests pass.",
          shown_by: ".armada/checks/77-split-the-settings-reducer/regression_verify.1.cargo_nextest.log",
        },
      ]),
      diff: diffRead(
        FILES,
        [
          "--- a/packages/settings/src/selectors.ts",
          "+++ b/packages/settings/src/selectors.ts",
          "@@ -14,6 +14,9 @@",
          "+import { selectColumnOrder } from './selectors/columns'",
        ].join("\n"),
      ),
      remarks: remarksRead(PULL_REQUEST, REMARKS),
    }),
    calls: {},
    checkOutputs: {},
    frames: {},
    now: NOW,
  };
}

/**
 * `fix`, worked twice — the build Check failed on attempt 1 and passed on
 * attempt 2. **One Check that passed**, which is what the lead's count has to
 * read off it: the gate records a row per declared Check per attempt, so
 * attempt 1's failure is history rather than a second Check.
 */
function fixRetried(): StepDetail {
  return {
    ...advancedStep("fix", "Fix", 3, [BUILD_CHECK]),
    check_runs: [
      buildRun({
        attempt: 1,
        outcome: "failed",
        expected: "cargo build --workspace --locked exits 0",
        produced: "error[E0432]: unresolved import `crate::settings::selectors`",
        output_path: ".armada/checks/77-split-the-settings-reducer/fix.1.cargo_build.log",
      }),
      buildRun({ attempt: 2 }),
    ],
    attempts: [
      { attempt: 1, outcome: "retrying", why: "gate_failure", started_at: "2026-09-10T14:16:07Z", ended_at: "2026-09-10T14:19:02Z" },
      { attempt: 2, outcome: "advanced", started_at: "2026-09-10T14:19:02Z", ended_at: "2026-09-10T14:22:18Z" },
    ],
    verdicts: [verdict(1, "failed", "gate_failure"), verdict(2, "passed")],
    last_verdict: verdict(2, "passed"),
  };
}

/**
 * `regression_verify`, refused by its Judge and advanced by a person —
 * `fleet::overruling`, admitted on `gate_failure` and refused where a Check
 * failed, so every Check here passed and `c1` is still `not_met`.
 */
function regressionOverruled(): StepDetail {
  return {
    ...advancedStep("regression_verify", "Regression check", 4, [NEXTEST_CHECK]),
    judge_checks: [{ criteria: 2, gaming_check: true }],
    judged: [
      {
        attempt: 1,
        criterion_id: "c1",
        verdict: "not_met",
        expected: "packages/settings/src/selectors.ts imports no store type",
        produced: "The module still imports RootState, which is declared in the store's own file.",
        consequence: "The selectors cannot be tested without the store's types being constructible.",
      },
      { attempt: 1, criterion_id: "c2", verdict: "met" },
    ],
    overridden: true,
    verdicts: [verdict(1, "failed", "gate_failure")],
    last_verdict: verdict(1, "failed", "gate_failure"),
  };
}

/**
 * The same delivering gate over a record a person has already argued with: one
 * Check retried, one criterion refused and overruled.
 *
 * **Its own fixture rather than a variant of `reviewAtDelivery`.** That one is
 * the clean sign-off and its whole value is being clean; what the lead's count
 * has to survive is a Job whose evidence is not, and folding the two together
 * would lose whichever of them is not being read.
 */
export function reviewAfterAnOverrule(): JobFixture {
  const theJob = job("awaiting_review", { current_step_id: "land" });
  const steps = [
    reproStep(),
    rootCauseStep(),
    fixRetried(),
    regressionOverruled(),
    advancedStep("consumers", "Check the consumers still compile", 5, [BUILD_CHECK]),
    landAtTheGate(),
  ];
  const whole = detail(theJob, steps, {
    delivery: { commit: "3c07a94", pushed: "origin/fix/settings-split-selectors", pull_request: PULL_REQUEST },
  });

  const rows = [
    instructed("land", "2026-09-10T14:26:56Z", 6, "the branch is pushed and a pull request is open", "Land"),
    said("land", "2026-09-10T14:27:05Z", "Pushing the branch and opening a pull request against main."),
    droneEnded("land", "2026-09-10T14:27:12Z", 3, 60_000),
  ];

  return {
    name: "awaiting_review — a Check was retried and a refusal overruled behind the branch that is out",
    job: theJob,
    watched: watchedRead(whole),
    workflows: [workflow()],
    manifests: [manifest()],
    observed: observedEnded(rows, "drone_ended"),
    journalled: journalledWatching([
      note("2026-09-10T14:19:02Z", "cargo_build failed on attempt 1 of 3. The step was handed back.", {
        level: "warn",
        step: "fix",
      }),
      note(
        "2026-09-10T14:24:50Z",
        "Regression check's Judge refused criterion 01. Overruled: RootState is a type, not the store.",
        { step: "regression_verify" },
      ),
      note("2026-09-10T14:27:03Z", "Pull request opened against main.", { step: "land" }),
    ]),
    resources: holdsRead(resources("none", { processes: [], wrote_last_at: "2026-09-10T14:27:12.000Z" })),
    // **A read holding no remark**, not an absent read: this pull request has
    // no comments on it, and leaving the read out draws *Reading what people
    // wrote* over a fetch nothing is doing.
    recorded: foldedReads({ diff: diffRead(FILES), remarks: remarksRead(PULL_REQUEST, []) }),
    calls: {},
    checkOutputs: {},
    frames: {},
    now: NOW,
  };
}
