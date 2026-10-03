// Job detail's fixtures beyond `fixtures/build`: one Job moved to a moment a
// test needs. Moved here from the `Screens/Job detail` stories that built
// them — #1224.

import type {
  ClaimedBreakage,
  DeclaredCheck,
  DeclaredJudge,
  JobSummary,
  PlanTask,
  Refusal,
  StepDetail,
  WorkflowSummary,
} from "@armada/protocol";
import type { Outstanding } from "@armada/screens/src/outstanding";
import { escalatedEvidenceSuspect, review } from "@armada/screens/src/fixtures/build/index";
import { awaitingApproval, running } from "@armada/screens/src/fixtures/build/index";
import { advancedStep, BUILD_CHECK, diffRead, freshStep, watchedRead } from "@armada/screens/src/fixtures/build/base";
import type { JobFixture } from "@armada/screens/src/fixtures/fixture";
import { recorded } from "@armada/screens/src/fixtures/recorded";

/**
 * The same gate as `Review`, holding on a judge question instead of a clean
 * pass. The question outranks the rest of the slot: no merge answer, no
 * checks list, just the criterion and the three presses.
 */
export function reviewAtAQuestion(): JobFixture {
  const fixture = review();
  if (fixture.watched.state !== "read") return fixture;
  return {
    ...fixture,
    watched: watchedRead({
      ...fixture.watched.detail,
      judge_question: {
        step_id: "regression_verify",
        criterion_id: "c1",
        question: "Does the fix address the cause the note names?",
        expected: "packages/settings/src/selectors.ts imports no store type",
        produced: "The module still imports RootState directly, behind a re-export",
        consequence: "the regression this step exists to catch can still reach the selectors",
        asked_at: "2026-09-10T14:29:40Z",
      },
    }),
  };
}

export const BROKEN = "settings::selectors::visible_manifests_memoises";

/** The Job fixing `BROKEN`, by its id and its title. */
export const FIX_JOB = "01M1FIXJOB000000000000000000";
export const FIX_TITLE = "Fix the selectors test broken on main";

/**
 * `BROKEN`, claimed by `fix`, as `whole.breakages` carries it at protocol
 * 23.3: under `cargo_nextest`, which `retryingCheckFailure` fails, with the
 * test's file held off every Job on the claim but the fix, and two Jobs
 * parked on it.
 */
export function brokenOnMain(fix: string, reportedBy: string): ClaimedBreakage {
  return {
    check: "cargo_nextest",
    test: BROKEN,
    failure: "expected the same reference on repeat calls",
    fix,
    fix_title: FIX_TITLE,
    reported_by: reportedBy,
    reported_by_title: "Trim the brief to the files the step touched",
    waiting: [
      { job_id: "01M1WAITINGONE00000000000000", title: "Round the cost estimate to a cent" },
      { job_id: "01M1WAITINGTWO00000000000000", title: "Memoise the manifest list" },
    ],
    held_off: ["packages/settings/src/selectors.ts"],
  };
}

/** A fixture — `running` where none is given — with claimed breakages on its detail. #1001, #1673. */
export function withBreakages(
  breakages: (jobId: string) => ClaimedBreakage[],
  fixture: JobFixture = running(),
): JobFixture {
  if (fixture.watched.state !== "read") return fixture;
  const { detail } = fixture.watched;
  return {
    ...fixture,
    watched: { ...fixture.watched, detail: { ...detail, breakages: breakages(fixture.job.id) } },
  };
}

/** A fixture with its row, and the detail's copy of it, changed alike. */
export function withRow(fixture: JobFixture, over: Partial<JobSummary>): JobFixture {
  const job = { ...fixture.job, ...over };
  if (fixture.watched.state !== "read") return { ...fixture, job };
  return { ...fixture, job, watched: watchedRead({ ...fixture.watched.detail, job }) };
}

/** `Fix`, running rather than finished — `running.ts`'s own shape, for the step taking over. */
export function runningStep(id: string, label: string, place: number): StepDetail {
  return {
    ...freshStep(id, label, place),
    state: "running",
    attempts: [{ attempt: 1, outcome: "running", started_at: "2026-09-10T14:30:00Z" }],
    entered_at: "2026-09-10T14:30:00Z",
    updated_at: "2026-09-10T14:30:00Z",
  };
}

/**
 * `running()`, one step on: Fix is done and Regression check is running —
 * the `job.step_advanced` Fleet would send, built by hand because these
 * stories are what a socket lands rather than a Job in a new state.
 */
export function advancedOnce(fixture: JobFixture): JobFixture {
  const whole = fixture.watched.state === "read" ? fixture.watched.detail : undefined;
  if (whole === undefined) return fixture;
  const steps = whole.steps.map((step) => {
    if (step.step_id === "fix") return advancedStep("fix", "Fix", 3, [BUILD_CHECK]);
    if (step.step_id === "regression_verify") return runningStep("regression_verify", "Regression check", 4);
    return step;
  });
  const job = { ...fixture.job, current_step_id: "regression_verify" };
  return { ...fixture, job, watched: watchedRead({ ...whole, job, steps }) };
}

export const TEST_FILE = "packages/settings/test/useColumnSelectors.test.ts";

/** A removed assertion, so the flag has a file and no line. */
export const PATCH = [
  `diff --git a/${TEST_FILE} b/${TEST_FILE}`,
  `--- a/${TEST_FILE}`,
  `+++ b/${TEST_FILE}`,
  "@@ -52,6 +52,5 @@",
  '   it("drops a column that was hidden", () => {',
  '     const next = reducer(state, hide("owner"));',
  '-    expect(selectVisible(next)).not.toContain("owner");',
  "     expect(next.version).toBe(state.version + 1);",
  "   });",
].join("\n");

export function refusedCommand(call: string, detail: string): Refusal {
  return { tool: "Bash", call, detail, truncated: false, because: "", offers: [], rules: [] };
}

/** The step's gaming check, with every pattern it looks for. Since protocol 13.50. */
export const GAMING: DeclaredJudge = {
  criteria: 2,
  panel_size: 3,
  gaming_check: true,
  gaming_patterns: ["assertion_weakened", "test_scope_narrowed", "tautological_test", "test_skipped", "test_deleted", "check_config_edited"],
};

/**
 * The owner's Job on 14 Sep, in the fixture's words: every Check passed, the
 * gaming check flagged a removed assertion, and three refused commands sat in
 * the same box. `recourse` is Fleet's reading of whether the Drone is still
 * there, which decides what Send it back is.
 */
export function heldByTheGamingCheck(recourse: string[]): JobFixture {
  const fixture = escalatedEvidenceSuspect();
  if (fixture.watched.state !== "read") return fixture;
  const whole = fixture.watched.detail;
  const steps = whole.steps.map(
    (step): StepDetail =>
      step.step_id !== "regression_verify"
        ? step
        : {
            ...step,
            judge_checks: [GAMING],
            judged: step.judged.map((one) => ({ ...one, verdict: "met" })),
            flagged: [
              {
                attempt: 1,
                pattern: "assertion_weakened",
                cited: '`expect(selectVisible(next)).not.toContain("owner")` was taken out, and nothing replaces it.',
                at: { file: TEST_FILE },
                asked:
                  "Does this change alter an existing assertion so that it asserts less than it did, " +
                  "and is that assertion made nowhere else in this change?",
                brief_path: ".armada/briefs/77-split-the-settings-reducer/regression_verify.1.gaming.txt",
              },
            ],
          },
  );
  return {
    ...fixture,
    // `Ruling::Suspect` needs a Judge that refused nothing, which the base's name denies.
    name: "escalated · evidence_suspect — every Check passed, the Judge met both criteria, and a flag stood",
    watched: watchedRead({
      ...whole,
      steps,
      stuck: {
        ...whole.stuck!,
        recourse,
        refused: [
          refusedCommand("call_1", "cargo nextest run --package fleet 2>&1 | tail -80"),
          refusedCommand("call_2", "git stash"),
          refusedCommand("call_3", "git log --oneline -20"),
        ],
        refusals: 3,
      },
    }),
    recorded: {
      ...fixture.recorded,
      diff: diffRead([{ path: TEST_FILE, change: "modified" }], PATCH),
    },
  };
}

/**
 * The running Job, changed: a model chosen for its later steps and one command
 * allowed for it. What the header counts, and what the panel lists.
 */
export function runningWithSettings(): JobFixture {
  const fixture = running();
  if (fixture.watched.state !== "read") return fixture;
  return {
    ...fixture,
    watched: watchedRead({
      ...fixture.watched.detail,
      model_override: "opus",
      allowed_commands: [
        {
          run: "pnpm add -D reselect@5.1.1",
          reach: "job",
          allowed_at: "2026-09-10T14:29:40Z",
          by: "human",
        },
      ],
      // Fleet's own table, since protocol 13.5 — covers every job against
      // this repository, so the panel draws it read-only. #836's own case:
      // `gh issue view`, always-allowed from Job 7 while filing #834.
      repository_allowed_commands: [
        {
          run: "gh issue view",
          reach: "repository",
          allowed_at: "2026-09-13T09:41:00Z",
          by: "human",
        },
      ],
    }),
  };
}

/**
 * A `refactor` Job held at its plan, **in the shape `GET /jobs/:job_id` served
 * for the owner's Job 1 on 1 Oct 2026**: three steps whose `ordinal` counts
 * from 0, the plan recorded at `plan` and waiting on a person, nothing entered
 * after it. Built on a recording, so every read around the detail is one a
 * Fleet served too.
 */
export function refactorAtItsPlan(): JobFixture {
  const base = recorded("done-worktree-given-back");
  // A step nothing has entered carries the Job's creation as both times.
  const created = "2026-10-01T18:29:37.596Z";
  const at = "2026-10-01T18:34:11.365Z";
  const until = "2026-10-01T18:36:51.494Z";
  const step = (step_id: string, label: string, ordinal: number, advance_gate: string): StepDetail => ({
    step_id,
    label,
    ordinal,
    state: "not_started",
    checks: [],
    check_runs: [],
    judge_checks: [{ criteria: 1, gaming_check: false }],
    advance_gate,
    delivers: false,
    overridden: false,
    judged: [],
    flagged: [],
    attempts: [],
    verdicts: [],
    entered_at: created,
    updated_at: created,
  });
  const plan: StepDetail = {
    ...step("plan", "Scope the refactor", 0, "auto_if_judge_passes"),
    state: "awaiting_human",
    checks: [{ kind: "plan_recorded" }],
    check_runs: [{ attempt: 1, name: "plan_recorded", outcome: "passed" }],
    attempts: [{ attempt: 1, outcome: "awaiting_human", started_at: at, ended_at: until }],
    entered_at: at,
    updated_at: until,
  };
  if (base.watched.state !== "read") return base;
  const job = {
    ...base.job,
    title: "Retire guide 8 and add guide validation rule",
    workflow_id: "refactor",
    status: "awaiting_review",
    current_step_id: "plan",
  };
  // Under the recording's own id, which `watchedRead` would replace with the
  // built fixtures'.
  const detail = {
    ...base.watched.detail,
    job,
    steps: [
      plan,
      step("implement", "Restructure", 1, "auto_if_judge_passes"),
      step("handoff", "Review the change", 2, "human_always"),
    ],
    work_plan: {
      approach: "Retire guide 8 the way guide 11 was retired, then add the rule that every guide's piece is drawn.",
      recorded_by: { by: "step" as const, step_id: "plan", attempt: 1 },
      recorded_at: "2026-10-01T18:36:09.854Z",
      tasks: [{ id: "T1", title: "Remove guide 8 from the catalogue and retire its number", state: "open" }],
    },
  };
  return { ...base, job, watched: { ...base.watched, detail } };
}

/**
 * The same Job one moment earlier, at its dispatch gate: `refactor`'s three
 * steps with the gate each froze and the two criteria the proposer read out of
 * the request, as `GET /jobs/1` served them — and no branch and no draft, which
 * is every real Fleet until #1545.
 */
export function refactorAtApproval(): JobFixture {
  const base = awaitingApproval();
  if (base.watched.state !== "read") return base;
  const created = "2026-10-01T18:29:37.596Z";
  const step = (step_id: string, label: string, ordinal: number, advance_gate: string): StepDetail => ({
    step_id,
    label,
    ordinal,
    state: "not_started",
    checks: [],
    check_runs: [],
    judge_checks: [],
    advance_gate,
    delivers: false,
    overridden: false,
    judged: [],
    flagged: [],
    attempts: [],
    verdicts: [],
    entered_at: created,
    updated_at: created,
  });
  const job = {
    ...base.job,
    handle: "1-retire-guide-8-and-add-guide-validation-ru",
    title: "Retire guide 8 and add guide validation rule",
    workflow_id: "refactor",
    origin: "auto_detected",
    model: "sonnet",
    current_step_id: "plan",
  };
  const steps: StepDetail[] = [
    {
      ...step("plan", "Scope the refactor", 0, "auto_if_judge_passes"),
      checks: [{ kind: "plan_recorded" }],
      judge_checks: [{ criteria: 1, gaming_check: false }],
    },
    {
      ...step("implement", "Restructure", 1, "auto_if_judge_passes"),
      checks: [{ kind: "every_manifest_check" }, { kind: "diff_nonempty" }],
      judge_checks: [{ criteria: 4, gaming_check: true }],
    },
    { ...step("handoff", "Review the change", 2, "human_always"), delivers: true },
  ];
  const detail = {
    ...base.watched.detail,
    job,
    steps,
    acceptance_criteria: [
      { criterion_id: "c1", text: "Guide 8 is removed from the catalogue", source: "judge" },
      {
        criterion_id: "c2",
        text: "A validation rule prevents guides without drawn pieces",
        source: "judge",
      },
    ],
  };
  // `refactor` as `GET /workflows` lists it since 21.1, with the line its
  // definition declares — and the same id under another repository, whose
  // line is not this Job's to read.
  const refactor = (manifest_id: string, for_requests: string): WorkflowSummary => ({
    id: "refactor",
    name: "refactor",
    version: 1,
    manifest_id,
    steps: steps.map((one) => ({
      step_id: one.step_id,
      label: one.label,
      checks: one.checks ?? [],
      judge_checks: one.judge_checks ?? [],
      advance_gate: one.advance_gate ?? "human_always",
      delivers: one.delivers ?? false,
    })),
    for_requests,
  });
  return {
    ...base,
    job,
    watched: watchedRead(detail),
    workflows: [
      ...base.workflows,
      refactor(job.owner_manifest_id, REFACTOR_FOR_REQUESTS),
      refactor("elsewhere", "Another repository's refactor, for another kind of request."),
    ],
  };
}

/** `.armada/workflows/refactor.json`'s `for_requests`, as that file declares it. */
export const REFACTOR_FOR_REQUESTS =
  "Reorganising code without changing what it does: moving, renaming, splitting or tidying, " +
  "where nothing anyone can see or use changes -- no screen, no output, no behaviour. A " +
  "request that adds, removes or changes anything a person sees or can do, however small, is " +
  "a feature and not a refactor.";

/**
 * `implement`'s Checks, as `GET /jobs/2` served them on 1 Oct 2026, commands
 * left out. So is `hooks_test`, whose one path names a vendor's directory and
 * which reaches none of these tasks.
 */
const JOB_2_CHECKS: DeclaredCheck[] = [
  { kind: "every_manifest_check" },
  { kind: "manifest_check", name: "build", when: ["crates/**", "xtask/**", "Cargo.toml", "Cargo.lock", ".cargo/**", "protocol-version.toml", ".armada/workflows/**", "armada.yml"] },
  { kind: "manifest_check", name: "test", when: ["crates/**", "xtask/**", "apps/**", "packages/**", "Cargo.toml", "Cargo.lock", ".cargo/**", "protocol-version.toml", ".armada/workflows/**", "armada.yml"] },
  { kind: "manifest_check", name: "acceptance", when: ["crates/**", "Cargo.toml", "Cargo.lock"] },
  { kind: "manifest_check", name: "typecheck", when: ["apps/**", "packages/**", "crates/core-model/domain/**", "protocol-version.toml", "package.json", "pnpm-lock.yaml", "pnpm-workspace.yaml"] },
  { kind: "manifest_check", name: "bridge_build", when: ["apps/**", "packages/**", "crates/core-model/domain/**", "protocol-version.toml", "package.json", "pnpm-lock.yaml", "pnpm-workspace.yaml"] },
  { kind: "manifest_check", name: "storybook", when: ["packages/**", "package.json", "pnpm-lock.yaml", "pnpm-workspace.yaml"], runs_at: "gate" },
  { kind: "manifest_check", name: "desktop_test", when: ["apps/**", "packages/**", "crates/core-model/domain/**", "protocol-version.toml", "package.json", "pnpm-lock.yaml", "pnpm-workspace.yaml"] },
  { kind: "manifest_check", name: "screens_test", when: ["packages/**", "crates/core-model/domain/**", "protocol-version.toml", "package.json", "pnpm-lock.yaml", "pnpm-workspace.yaml"] },
  { kind: "manifest_check", name: "components_test", when: ["packages/**", "crates/core-model/domain/**", "protocol-version.toml", "package.json", "pnpm-lock.yaml", "pnpm-workspace.yaml"], runs_at: "gate" },
  { kind: "manifest_check", name: "scripts_test", when: ["scripts/**", "crates/armada/src/land/**", "crates/adapter-traits/src/delivery.rs", "crates/adapters/src/delivery.rs", "crates/adapters/src/landing.rs", "armada.yml"] },
  { kind: "manifest_check", name: "format", when: ["**/*.rs", "Cargo.toml", "rustfmt.toml", "armada.yml"] },
  { kind: "diff_nonempty" },
];

/**
 * A `feature` Job on its plan step, **in the shape `GET /jobs/2` served for the
 * owner's Job 2 on 1 Oct 2026**: the plan recorded at `plan`, four tasks every
 * one `open`, no groups, and the Job's one Drone on the step — `assigned_drone`
 * on the row, named by no task. `tasks` moves a task on, as a later read would.
 */
export function featureOnItsPlan(tasks: Partial<Record<string, PlanTask["state"]>> = {}): JobFixture {
  const base = recorded("done-worktree-given-back");
  const created = "2026-10-01T20:22:43.311Z";
  const at = "2026-10-01T20:23:51.575Z";
  const step = (step_id: string, label: string, ordinal: number, checks: DeclaredCheck[]): StepDetail => ({
    step_id,
    label,
    ordinal,
    state: "not_started",
    checks,
    check_runs: [],
    judge_checks: [{ criteria: 1, gaming_check: false }],
    advance_gate: "auto_if_judge_passes",
    delivers: false,
    overridden: false,
    judged: [],
    flagged: [],
    attempts: [],
    verdicts: [],
    entered_at: created,
    updated_at: created,
  });
  const plan: StepDetail = {
    ...step("plan", "Plan the change", 0, [{ kind: "plan_recorded" }]),
    state: "running",
    attempts: [{ attempt: 1, outcome: "running", started_at: at }],
    entered_at: at,
    updated_at: at,
  };
  if (base.watched.state !== "read") return base;
  const task = (id: string, title: string, scope: string[]): PlanTask => ({
    id,
    title,
    scope,
    state: tasks[id] ?? "open",
  });
  const planned = [
    task("T1", "Remove guides 8 and 20 from the catalogue and retire their numbers", [
      "packages/components/src/guides/index.ts",
      "packages/components/src/guides/008-what-does-the-progress-bar-show.ts",
      "packages/components/src/guides/020-what-if-a-step-changes-a-file-it-never-said-it-would.ts",
    ]),
    task("T2", "Remove what only guide 8 used: the InsideAJob mark and the step-bar guide figure", [
      "packages/screens/src/InsideAJob.tsx",
      "packages/components/src/guides/guide.ts",
      "packages/components/src/compositions/GuideFigure/GuideFigure.tsx",
      "packages/components/src/compositions/GuideFigure/GuideFigure.stories.tsx",
    ]),
    task("T3", "Add the xtask rule that every guide's piece is drawn somewhere", [
      "xtask/src/rules_guides.rs",
      "xtask/src/main.rs",
    ]),
    task("T4", "Document the rule and fix prose that pointed at the retired guides", [
      "docs/contracts/design-system.md",
      "docs/INDEX.md",
      "docs/concepts",
    ]),
  ];
  const count = (state: string) => planned.filter((one) => one.state === state).length;
  const job = {
    ...base.job,
    title: "Retire guides 8 and 20, add validation that every guide's piece is drawn somewhere",
    workflow_id: "feature",
    status: "running",
    current_step_id: "plan",
    assigned_drone: "01M3WJ6FGZ003DCX123T7W6YP1",
    tasks: { done: count("done"), working: count("working"), open: count("open"), dropped: count("dropped") },
  };
  const detail = {
    ...base.watched.detail,
    job,
    steps: [
      plan,
      step("implement", "Implement", 1, JOB_2_CHECKS),
      step("tests", "Write tests", 2, JOB_2_CHECKS),
      { ...step("handoff", "Review the change", 3, []), advance_gate: "human_always", delivers: true },
    ],
    work_plan: {
      approach: "Retire guides 8 and 20 the way guide 11 was retired, then add the rule that every guide's piece is drawn.",
      recorded_by: { by: "step" as const, step_id: "plan", attempt: 1 },
      recorded_at: "2026-10-01T20:24:44.953Z",
      tasks: planned,
    },
  };
  return { ...base, job, watched: { ...base.watched, detail } };
}

/** The command a Job waits on, as Helm's dock lists it. */
export function commandOutstanding(fixture: JobFixture): Outstanding {
  const whole = fixture.watched.state === "read" ? fixture.watched.detail : undefined;
  const waiting = whole?.command_waiting;
  if (waiting === undefined) throw new Error("fixture carries no command_waiting to share");
  return { kind: "command", job_id: fixture.job.id, waiting };
}
