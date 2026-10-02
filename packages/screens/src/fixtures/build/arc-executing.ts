// Four moments inside `implement`: one group at a time, two tasks at once, a
// boundary that failed, and a finished task a later one edited.
//
// **The Checks run at each group's end** (#1530, 21 Sep), so a group carries
// the verdict, the commit and the retry count, and a task carries only its own
// agent's turns and cost.
//
// **A task's cost appears when that task's agent stops** (owner, 22 Sep 2026),
// which is why a done task in a group still being checked already shows one.
//
// **A done task a later task edits stays done and is flagged** (#1530). T6
// finished in group three; T7 writes the same file in group four.

import type { Diff, JobProcess, LogFile, Recorded, StepDetail, Turn } from "@armada/protocol";
import type { CaseRunView, CaseView, GroupView, LedgerRow, PulseView } from "../../draft";
import type { JobFixture } from "../fixture";
import type { ArcMoment } from "./arc-base";
import { arcDrones, t6Retry } from "./arc-drones";
import {
  ARC_APPROVED_AT,
  ARC_BRANCH,
  ARC_HANDLE,
  ARC_JOB_ID,
  ARC_NOW,
  ARC_TITLE,
  ARC_WORKTREE,
  arcAdvanced,
  arcCriterionViews,
  arcDetail,
  arcJob,
  arcManifests,
  arcResources,
  arcRunning,
  arcStep,
  arcSteps,
  arcWatched,
  arcWorkPlan,
  BRIDGE_CHECKS,
  checkNames,
  featureWorkflow,
  RUST_CHECKS,
  tasksOf,
} from "./arc-base";
import { ARC_LANDING } from "./arc-dispatch";
import { ARC_DRONES, arcCases, arcGroups, finished, withGroup, withTask } from "./arc-plan";
import { arcApproved } from "./arc-proposing";
import { answered, called, said } from "./base";
import { briefBytes, briefName, judgeBrief } from "./briefs";

const IMPLEMENT_ENTERED = "2026-09-22T09:22:00Z";

/** The plan step, behind every moment in this file. */
function planAdvanced(): StepDetail {
  return {
    ...arcAdvanced(arcStep("plan", "Plan the change", 1), "2026-09-22T09:15:00Z", "2026-09-22T09:21:00Z"),
    judge_checks: [{ criteria: 2, gaming_check: false }],
    judged: [
      { attempt: 1, criterion_id: "a1", verdict: "met", brief_path: PLAN_BRIEF("a1") },
      { attempt: 1, criterion_id: "a2", verdict: "met", brief_path: PLAN_BRIEF("a2") },
    ],
  };
}

/** Where the plan's Judge brief for one criterion was kept. */
const PLAN_BRIEF = (criterion: string) => `.armada/briefs/${ARC_HANDLE}/plan.1.${criterion}.md`;

/** The plan's scope note, as both of its Judge briefs lay it out. */
const PLAN_EVIDENCE = [
  "What this step produced, as the Drone handed it in:",
  "",
  "  scope: The rail's Drones stat reads \"1 running · 2 max\" and opens a panel listing what Fleet is running now: Drones, Checks, Judge calls and proposer calls, each with its Job and how long. Fleet's half is one read of everything running on the machine, and an event when it changes.",
  "  tasks: 8, in four groups. T1 adds the read and its event; T2 to T4 the panel's sections; T5 the stat's wording; T6 to T8 the stories.",
  "  not claimed: No code was changed in this part — planning only.",
];

/** What Fleet kept for each of the plan's two criteria. */
const PLAN_BRIEFS = Object.fromEntries(
  (
    [
      ["a1", "Does this scope note address what was actually requested, without expanding beyond it?"],
      ["a2", "Does the plan name every file and route it will touch, so a reviewer can tell a change it did not announce?"],
    ] as const
  ).map(([criterion, question]) => [
    briefName(PLAN_BRIEF(criterion)),
    judgeBrief(PLAN_BRIEF(criterion), {
      step: "Plan the change",
      request: ARC_TITLE,
      said: "The rail's Drones stat says \"1 of 2\", and nothing shows what is running.",
      evidence: PLAN_EVIDENCE,
      question,
    }),
  ]),
);

/**
 * The Drone that wrote the plan. Its id sorts before every task's, as a ULID
 * minted first does, so Fleet lists its transcript first.
 */
const PLAN_DRONE = "01M2D5HKQN001DRONE00PLAN";

/** The transcript of one task's Drone, or the plan's, as the reading lists it. */
function transcript(task: string, bytes: number | undefined, writing: boolean | undefined): LogFile {
  return {
    kind: "transcript",
    path: `.armada/transcripts/${ARC_HANDLE}/${task === "plan" ? PLAN_DRONE : (ARC_DRONES[task] ?? task)}.jsonl`,
    ...(bytes === undefined ? {} : { bytes }),
    ...(writing === undefined ? {} : { being_written: writing }),
  };
}

/**
 * Every file the Job has by group three: its own log and T5's transcript held
 * open, the plan's and four tasks' finished transcripts, and the plan's two
 * briefs.
 *
 * **T3's would not `stat`**, so it has no size and no answer about a writer:
 * with no inode to match, Fleet learns neither.
 */
function sequentialLogs(): LogFile[] {
  return [
    { kind: "job", path: `.armada/logs/${ARC_HANDLE}.jsonl`, bytes: 96_412, being_written: true },
    transcript("plan", 842_551, false),
    transcript("T1", 1_288_304, false),
    transcript("T2", 402_118, false),
    transcript("T3", undefined, undefined),
    transcript("T4", 517_930, false),
    transcript("T5", 611_205, true),
    ...["a1", "a2"].map((criterion) => ({
      kind: "brief" as const,
      path: PLAN_BRIEF(criterion),
      bytes: briefBytes(PLAN_BRIEFS[briefName(PLAN_BRIEF(criterion))]) ?? 0,
      being_written: false,
    })),
  ];
}

/**
 * Each Drone arriving and leaving, as the Job's history records it: the plan's,
 * then one per task through T5, which is still on `implement`. What names a
 * transcript row by its step.
 */
function sequentialHistory(): Recorded[] {
  const drones: [string, string, string, string | undefined][] = [
    ["plan", PLAN_DRONE, "2026-09-22T09:15:00Z", "2026-09-22T09:21:00Z"],
    ["implement", ARC_DRONES.T1 ?? "T1", "2026-09-22T09:22:00Z", "2026-09-22T09:34:00Z"],
    ["implement", ARC_DRONES.T2 ?? "T2", "2026-09-22T09:34:00Z", "2026-09-22T09:46:00Z"],
    ["implement", ARC_DRONES.T3 ?? "T3", "2026-09-22T09:50:00Z", "2026-09-22T09:58:00Z"],
    ["implement", ARC_DRONES.T4 ?? "T4", "2026-09-22T09:58:00Z", "2026-09-22T10:08:00Z"],
    ["implement", ARC_DRONES.T5 ?? "T5", "2026-09-22T10:14:00Z", undefined],
  ];
  const moves = drones.flatMap(([step, drone, spawned, exited]) => [
    { step, drone, presence: "drone_spawned", at: spawned },
    ...(exited === undefined ? [] : [{ step, drone, presence: "drone_exited", at: exited }]),
  ]);
  return moves.map((one, seq) => ({
    seq: seq + 1,
    status: "running",
    moved: { kind: "drone", step_id: one.step, drone_id: one.drone, presence: one.presence },
    actor: "fleet",
    at: one.at,
  }));
}

/** One row of T5's transcript, stamped with its Drone as Fleet stamps it. */
function byT5(row: Turn): Turn {
  return { ...row, drone_id: ARC_DRONES.T5 ?? "T5" };
}

/**
 * T5 at work in group three: what the observe socket opened with, and the
 * rows it carries after, which the mock lands one at a time.
 */
function t5Transcript(): { opened: Turn[]; arriving: Turn[] } {
  return {
    opened: [
      said("implement", "2026-09-22T10:14:20Z", "Reading the stat to see where the Drone count is drawn."),
      called("implement", "2026-09-22T10:15:02Z", "call_t5_read", "Read", "packages/screens/src/running-rows.tsx"),
      answered("implement", "2026-09-22T10:15:03Z", "call_t5_read"),
      said("implement", "2026-09-22T10:17:40Z", "The rows take the read's lists as they are. Adding the Judge calls row."),
      called("implement", "2026-09-22T10:18:55Z", "call_t5_edit", "Edit", "packages/screens/src/running-rows.tsx +14 -2"),
      answered("implement", "2026-09-22T10:18:56Z", "call_t5_edit"),
    ].map(byT5),
    arriving: [
      said("implement", "2026-09-22T10:20:10Z", "Running the screens tests against the new row."),
      called("implement", "2026-09-22T10:20:12Z", "call_t5_test", "Bash", "armada check screens_test"),
    ].map(byT5),
  };
}

/** One Drone's process, as `ps` reports it. */
function droneProcess(pid: number, ran: string): JobProcess {
  return {
    pid,
    command: "node",
    cpu_percent: 12.4,
    memory_bytes: 486_539_264,
    running_for: ran,
    recorded: true,
  };
}

/** Everything the Job is holding, with a process per Drone that is up. */
function pulse(readAt: string, processes: JobProcess[]): PulseView {
  return {
    job: ARC_JOB_ID,
    read_at: readAt,
    held: processes.length === 0 ? "none" : "running",
    processes: processes.map((one) => ({
      pid: one.pid,
      command: one.command,
      cpu_percent: one.cpu_percent,
      memory_bytes: one.memory_bytes,
      running_for: one.running_for,
      recorded: one.recorded,
      // Concurrent tasks share one checkout, so every process here is placed
      // in the Job's own worktree. A second worktree is what a member would
      // bring, and `implement` has none.
      owner: ARC_BRANCH,
    })),
    worktrees: [{ path: ARC_WORKTREE, branch: ARC_BRANCH, bytes: 1_020_054_016 }],
    logs: [{ kind: "job", path: `.armada/logs/${ARC_HANDLE}.jsonl`, owner: null, writing: processes.length > 0 }],
  };
}

/**
 * The Job's diff once group three's agents stopped — every file its tasks
 * wrote, and the one T1 wrote that the Job never named. A written file with no
 * section here would read on the Record as a file the branch never changed.
 */
const ARC_PATCH = [
  "diff --git a/packages/screens/src/Running.tsx b/packages/screens/src/Running.tsx",
  "new file mode 100644",
  "--- /dev/null",
  "+++ b/packages/screens/src/Running.tsx",
  "@@ -0,0 +1,14 @@",
  '+import { RunningList } from "@armada/components";',
  '+import type { Running } from "@armada/protocol";',
  "+",
  "+/** What is running, in four lists: Drones, Checks, Judge calls, proposer calls. */",
  "+export function RunningPanel({ running }: { running: Running }) {",
  "+  return (",
  '+    <div className="armada-running">',
  '+      <RunningList title="Drones" rows={running.drones} />',
  '+      <RunningList title="Checks" rows={running.checks} />',
  '+      <RunningList title="Judge calls" rows={running.judges} />',
  '+      <RunningList title="Proposer calls" rows={running.proposers} />',
  "+    </div>",
  "+  );",
  "+}",
  "diff --git a/crates/api/src/running.rs b/crates/api/src/running.rs",
  "new file mode 100644",
  "--- /dev/null",
  "+++ b/crates/api/src/running.rs",
  "@@ -0,0 +1,6 @@",
  "+//! `GET /running` — everything running, in one read.",
  "+",
  "+pub async fn get_running(State(fleet): State<Fleet>) -> Json<Running> {",
  "+    Json(fleet.running().await)",
  "+}",
  "+",
  "diff --git a/crates/fleet/src/running.rs b/crates/fleet/src/running.rs",
  "--- a/crates/fleet/src/running.rs",
  "+++ b/crates/fleet/src/running.rs",
  "@@ -41,6 +41,9 @@ impl Fleet {",
  "     pub fn spawn_drone(&self, job: &JobId) -> DroneHandle {",
  "         let handle = self.drones.spawn(job);",
  "+        self.events.send(Event::RunningChanged);",
  "         handle",
  "     }",
  "+",
  "diff --git a/packages/screens/src/overview.ts b/packages/screens/src/overview.ts",
  "--- a/packages/screens/src/overview.ts",
  "+++ b/packages/screens/src/overview.ts",
  "@@ -88,7 +88,7 @@ export function dronesStat(capacity: FleetCapacity): Stat {",
  "   return {",
  '     label: "Drones",',
  "-    value: `${capacity.reported}`,",
  "+    value: `${capacity.running} running of ${capacity.max}`,",
  "   };",
  " }",
  "diff --git a/packages/screens/src/Board.tsx b/packages/screens/src/Board.tsx",
  "--- a/packages/screens/src/Board.tsx",
  "+++ b/packages/screens/src/Board.tsx",
  "@@ -140,4 +140,4 @@ function Stats({ stats }: StatsProps) {",
  "-      <Stat label=\"Drones\" value={stats.drones} hint=\"reported\" />",
  "+      <Stat label=\"Drones\" value={stats.drones} />",
  "diff --git a/packages/screens/src/SettingsSurface.tsx b/packages/screens/src/SettingsSurface.tsx",
  "--- a/packages/screens/src/SettingsSurface.tsx",
  "+++ b/packages/screens/src/SettingsSurface.tsx",
  "@@ -212,3 +212,3 @@ function DroneCap({ cap }: DroneCapProps) {",
  "-      <p>At most {cap} drones report at once.</p>",
  "+      <p>At most {cap} running at once.</p>",
  "diff --git a/packages/screens/src/running-rows.tsx b/packages/screens/src/running-rows.tsx",
  "new file mode 100644",
  "--- /dev/null",
  "+++ b/packages/screens/src/running-rows.tsx",
  "@@ -0,0 +1,5 @@",
  "+/** A Drone's row, pressable: it opens the Job the Drone is working. */",
  "+export function droneRowOpens(row: RunningDrone): string {",
  "+  return row.job_id;",
  "+}",
  "+",
  "diff --git a/crates/ipc/operations.toml b/crates/ipc/operations.toml",
  "--- a/crates/ipc/operations.toml",
  "+++ b/crates/ipc/operations.toml",
  "@@ -212,5 +212,11 @@ path = \"/jobs/:job_id/resources\"",
  ' method = "GET"',
  ' answers = "JobResources"',
  " ",
  "+[[operation]]",
  '+name = "get_running"',
  '+path = "/running"',
  '+method = "GET"',
  '+answers = "Running"',
  "+",
  " [[operation]]",
  ' name = "get_worktrees"',
].join("\n");

const ARC_DIFF: Diff = {
  state: "read",
  jobId: ARC_JOB_ID,
  work: {
    files: [
      { path: "packages/screens/src/Running.tsx", change: "added" },
      { path: "crates/api/src/running.rs", change: "added" },
      { path: "crates/fleet/src/running.rs", change: "modified" },
      { path: "packages/screens/src/overview.ts", change: "modified" },
      { path: "packages/screens/src/Board.tsx", change: "modified" },
      { path: "packages/screens/src/SettingsSurface.tsx", change: "modified" },
      { path: "packages/screens/src/running-rows.tsx", change: "added" },
      { path: "crates/ipc/operations.toml", change: "modified", outside_plan: true },
    ],
    measured_from: "main",
    measured_whole: true,
    plan_declared: true,
    patch: ARC_PATCH,
  },
};

/** The Job at some instant inside `implement`. */
function executing(args: {
  says: string;
  groups: GroupView[];
  step: StepDetail;
  processes: JobProcess[];
  status?: string;
  /** The Job's diff, where this moment serves one. */
  diff?: Diff;
  /** The files the reading lists, where this moment serves them. */
  logs?: LogFile[];
  /** The Job's history, where this moment serves one. */
  history?: Recorded[];
  /** What the observe socket opened with, and what it carries after. */
  transcript?: { opened: Turn[]; arriving: Turn[] };
}): JobFixture {
  const job = arcJob(args.status ?? "running", {
    current_step_id: "implement",
    started_at: ARC_APPROVED_AT,
    assigned_drone: ARC_DRONES.T5,
  });
  const steps = [planAdvanced(), args.step, ...arcSteps().slice(2)];
  const groups = args.groups;
  const whole = arcDetail(job, steps, { work_plan: arcWorkPlan(tasksOf(groups)) });
  return {
    name: args.says,
    job,
    watched: arcWatched(whole),
    workflows: [featureWorkflow()],
    manifests: arcManifests(),
    observed:
      args.transcript === undefined
        ? { state: "none" }
        : {
            state: "watching",
            jobId: ARC_JOB_ID,
            turns: { live: true, skipped: 0, missed: 0, rows: args.transcript.opened },
          },
    ...(args.transcript === undefined ? {} : { arriving: args.transcript.arriving }),
    journalled: { state: "none" },
    resources: {
      state: "read",
      jobId: ARC_JOB_ID,
      resources: arcResources(args.processes.length === 0 ? "none" : "running", args.processes, args.logs),
    },
    recorded: {
      footprint: { state: "none" },
      handed: { state: "none" },
      evidence: { state: "none" },
      diff: args.diff ?? { state: "none" },
      remarks: { state: "none" },
    },
    ...(args.history === undefined ? {} : { history: { state: "read", jobId: ARC_JOB_ID, moves: args.history } }),
    checkOutputs: {},
    briefs: PLAN_BRIEFS,
    frames: {},
    now: ARC_NOW,
  };
}

/** `implement`, mid-run, with whatever the last boundary's Checks came to. */
function implementStep(runs: StepDetail["check_runs"], at: string): StepDetail {
  const step = arcRunning(
    arcStep("implement", "Implement", 2, [...RUST_CHECKS, ...BRIDGE_CHECKS]),
    IMPLEMENT_ENTERED,
    at,
  );
  return { ...step, check_runs: runs };
}

/** Every Check a boundary ran, all passed. */
function allPassed(names: string[], attempt = 1): StepDetail["check_runs"] {
  return names.map((name) => ({ attempt, name, outcome: "passed" }));
}

/** The first two groups, passed, with what their agents cost. */
function throughGroupTwo(): GroupView[] {
  let groups = arcGroups();
  groups = withTask(groups, "T1", {
    ...finished(34, 2_400_000, "The read answers Drones, Checks and Judge calls in one call"),
    drone_id: ARC_DRONES.T1,
  });
  groups = withTask(groups, "T2", {
    ...finished(12, 640_000, "A Drone starting moves the read without a poll"),
    drone_id: ARC_DRONES.T2,
  });
  groups = withTask(groups, "T3", {
    ...finished(8, 260_000, "The stat reads one running and two at most"),
    drone_id: ARC_DRONES.T3,
  });
  groups = withTask(groups, "T4", {
    ...finished(9, 310_000, "Neither surface spells the pair with of"),
    drone_id: ARC_DRONES.T4,
  });
  groups = withGroup(groups, "g1", {
    state: "passed",
    verdict: "passed",
    commit: "4c1b9d2",
  });
  return withGroup(groups, "g2", { state: "passed", verdict: "passed", commit: "7a2f0c5" });
}

/** The Record up to the end of group two, a Check and a Judge each on a row. */
function recordThroughGroupTwo(): LedgerRow[] {
  return [
    {
      at: "2026-09-22T09:41:00Z",
      coord: { step: "implement", step_attempt: 1, group: "g1", group_attempt: 1, task: "T1" },
      actor: "drone",
      kind: "drone_exited",
      what: "T1's agent stopped",
      outcome: "34 turns, and what it cost",
      cursor: 8,
    },
    {
      at: "2026-09-22T09:56:00Z",
      coord: { step: "implement", step_attempt: 1, group: "g1", group_attempt: 1 },
      actor: "check",
      kind: "checked",
      what: "four Checks at group one's boundary",
      outcome: "all four passed",
      cursor: 9,
    },
    {
      at: "2026-09-22T10:14:00Z",
      coord: { step: "implement", step_attempt: 1, group: "g2", group_attempt: 1 },
      actor: "check",
      kind: "checked",
      what: "seven Checks at group two's boundary",
      outcome: "all seven passed",
      cursor: 10,
    },
  ];
}

export function executingSequential(): ArcMoment {
  let groups = throughGroupTwo();
  groups = withGroup(groups, "g3", { state: "running" });
  groups = withTask(groups, "T5", { state: "working", turns: 14, drone_id: ARC_DRONES.T5 });
  return {
    name: "executingSequential",
    says: "Implement — groups one and two passed, group three is working",
    fixtures: [
      executing({
        says: "running — one group at a time, the third of four working",
        groups,
        step: implementStep(allPassed(checkNames(BRIDGE_CHECKS)), "2026-09-22T10:20:00Z"),
        processes: [droneProcess(52_118, "06:12")],
        logs: sequentialLogs(),
        history: sequentialHistory(),
        transcript: t5Transcript(),
      }),
    ],
    opens: ARC_JOB_ID,
    draft: {
      groups,
      cases: arcCases(),
      criteria: arcCriterionViews(),
      // What the gate settled and froze — the Drone cap a concurrent group is
      // bounded by, among the rest. `#1550`.
      proposal: arcApproved(),
      landing: ARC_LANDING,
      record: recordThroughGroupTwo(),
      drones: arcDrones(groups),
      pulse: pulse("2026-09-22T10:20:00.000Z", [droneProcess(52_118, "06:12")]),
    },
  };
}

/**
 * The cases, once groups one and two have passed their boundaries: the API's
 * case ran at group one's, and the Board's has no spec to run. The panel's and
 * the overview's run at group four's, which nothing has reached yet.
 */
function casesRunThroughGroupTwo(): CaseView[] {
  const runs: Record<string, CaseRunView> = {
    "c-api": {
      id: "run-g1-c-api",
      case: "c-api",
      coord: { step: "implement", step_attempt: 1, group: "g1", group_attempt: 1 },
      actor: "fleet",
      purpose: "group_boundary",
      tree: "branch",
      outcome: "ran",
      frames: 0,
      ran_at: "2026-09-22T10:14:00Z",
    },
    "c-board": {
      id: "run-g2-c-board",
      case: "c-board",
      coord: { step: "implement", step_attempt: 1, group: "g2", group_attempt: 1 },
      actor: "fleet",
      purpose: "group_boundary",
      tree: "branch",
      outcome: "not_run",
      not_run_reason: "no spec covers Board.tsx",
      frames: 0,
      ran_at: "2026-09-22T10:31:00Z",
    },
  };
  return arcCases().map((one) => (runs[one.id] === undefined ? one : { ...one, last_run: runs[one.id]! }));
}

export function executingConcurrent(): ArcMoment {
  let groups = throughGroupTwo();
  groups = withTask(groups, "T5", {
    ...finished(27, 1_900_000, "The panel lists Drones, Checks, Judge calls and proposer calls"),
    drone_id: ARC_DRONES.T5,
  });
  groups = withTask(groups, "T6", {
    ...finished(15, 720_000, "Pressing a Drone's row opens **that Job**, in `running-rows.test.tsx`"),
    drone_id: ARC_DRONES.T6,
  });
  // Joining, not checking: both agents have stopped and their work is being
  // brought together before the boundary's Checks run. Each task's cost is
  // already on it, because each agent stopped.
  groups = withGroup(groups, "g3", { state: "joining" });
  return {
    name: "executingConcurrent",
    says: "Implement — two tasks ran at once, and group three is joining their work",
    fixtures: [
      executing({
        says: "running — two tasks of one group ran at the same time",
        groups,
        step: implementStep(allPassed(checkNames(BRIDGE_CHECKS)), "2026-09-22T10:38:00Z"),
        processes: [],
        diff: ARC_DIFF,
      }),
    ],
    opens: ARC_JOB_ID,
    draft: {
      groups,
      cases: casesRunThroughGroupTwo(),
      criteria: arcCriterionViews(),
      // What the gate settled and froze — the Drone cap a concurrent group is
      // bounded by, among the rest. `#1550`.
      proposal: arcApproved(),
      landing: ARC_LANDING,
      record: recordThroughGroupTwo(),
      drones: arcDrones(groups),
      pulse: pulse("2026-09-22T10:38:00.000Z", []),
    },
  };
}

/** What the failed screens_test printed, as Fleet's output route serves it. */
const SCREENS_TEST_LINES = [
  " FAIL  src/running.test.tsx > the Drones row opens the Job it is working",
  "AssertionError: expected 'board' to be 'job'",
  "  ❯ src/running.test.tsx:44:31",
  "",
  " Test Files  1 failed | 212 passed (213)",
  "      Tests  1 failed | 1383 passed (1384)",
];

const SCREENS_TEST_OUTPUT = {
  ok: true as const,
  output: {
    attempt: 1,
    name: "screens_test",
    path: ".armada/checks/3-show-what-s-running/implement.1.screens_test.log",
    lines: SCREENS_TEST_LINES,
    from_line: 1,
    total_lines: SCREENS_TEST_LINES.length,
    bytes: SCREENS_TEST_LINES.join("\n").length,
    whole: true,
  },
};

export function groupFailed(): ArcMoment {
  let groups = executingConcurrent().draft.groups!;
  groups = withTask(groups, "T6", {
    state: "failed",
    failed_reason:
      "The row's press opened the Board rather than the Job:\n\n" +
      "- `openBoard` ran on **every** row\n" +
      "- the Job's id was never read",
  });
  groups = withGroup(groups, "g3", {
    state: "retrying",
    verdict: "failed",
    retry_count: 1,
  });
  const runs = [
    ...allPassed(checkNames(BRIDGE_CHECKS).filter((name) => name !== "screens_test")),
    {
      attempt: 1,
      name: "screens_test",
      outcome: "failed",
      expected: "every test in the screens package passes",
      produced: "1 of 1384 failed: the Drones row opened the Board",
      output_path: ".armada/checks/3-show-what-s-running/implement.1.screens_test.log",
    },
  ];
  return {
    name: "groupFailed",
    says: "Implement — group three failed its Checks and is on its second run",
    fixtures: [
      {
        ...executing({
          says: "running — a group failed at its boundary and is being run again",
          groups,
          step: implementStep(runs, "2026-09-22T10:46:00Z"),
          processes: [droneProcess(52_640, "01:40")],
          diff: ARC_DIFF,
        }),
        checkOutputs: { "implement.1.screens_test.log": SCREENS_TEST_OUTPUT },
      },
    ],
    opens: ARC_JOB_ID,
    draft: {
      groups,
      cases: casesRunThroughGroupTwo(),
      criteria: arcCriterionViews(),
      // What the gate settled and froze — the Drone cap a concurrent group is
      // bounded by, among the rest. `#1550`.
      proposal: arcApproved(),
      landing: ARC_LANDING,
      record: [
        ...recordThroughGroupTwo(),
        {
          at: "2026-09-22T10:44:00Z",
          coord: { step: "implement", step_attempt: 1, group: "g3", group_attempt: 1 },
          actor: "check",
          kind: "checked",
          what: "seven Checks at group three's boundary",
          outcome: "screens_test failed — the Drones row opened the Board",
          cursor: 11,
        },
      ],
      drones: [...arcDrones(groups), t6Retry(groups)],
      pulse: pulse("2026-09-22T10:46:00.000Z", [droneProcess(52_640, "01:40")]),
    },
  };
}

export function doneTouched(): ArcMoment {
  let groups = executingConcurrent().draft.groups!;
  groups = withGroup(groups, "g3", {
    state: "passed",
    verdict: "passed",
    commit: "b81c3e4",
    retry_count: 1,
  });
  // T6 finished in group three. T7 writes the same file in group four, so T6
  // stays done and carries the flag — moving it back to `open` would lose the
  // fact that it was finished once.
  groups = withTask(groups, "T6", { touched_after_done: true });
  groups = withGroup(groups, "g4", { state: "running" });
  groups = withTask(groups, "T7", { state: "working", turns: 6, drone_id: ARC_DRONES.T7 });
  return {
    name: "doneTouched",
    says: "Implement — a finished task's file was edited by a later one, and it stays done",
    fixtures: [
      executing({
        says: "running — a later task edited a file a finished task had written",
        groups,
        step: implementStep(allPassed(checkNames(BRIDGE_CHECKS), 2), "2026-09-22T11:05:00Z"),
        processes: [droneProcess(53_402, "02:55")],
      }),
    ],
    opens: ARC_JOB_ID,
    draft: {
      groups,
      cases: casesRunThroughGroupTwo(),
      criteria: arcCriterionViews(),
      // What the gate settled and froze — the Drone cap a concurrent group is
      // bounded by, among the rest. `#1550`.
      proposal: arcApproved(),
      landing: ARC_LANDING,
      record: [
        ...recordThroughGroupTwo(),
        {
          at: "2026-09-22T11:02:00Z",
          coord: { step: "implement", step_attempt: 1, group: "g4", group_attempt: 1, task: "T7" },
          actor: "fleet",
          kind: "touched_after_done",
          what: "T7 wrote running-rows.tsx, which T6 had finished",
          outcome: "T6 stays done, and is flagged",
          cursor: 12,
        },
      ],
      drones: arcDrones(groups),
      pulse: pulse("2026-09-22T11:05:00.000Z", [droneProcess(53_402, "02:55")]),
    },
  };
}
