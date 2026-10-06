// One line of the Record: what happened, where, and what it came to. Draft,
// for `crates/ipc/src/history.rs`.
//
// Source of truth today: `JobHistory.moves` — `Recorded` with its `seq`,
// `status`, `actor`, `at` and one of three `Movement` shapes.
//
// **`kind` stays an opaque string** (#1532). A new kind of row is then a minor
// bump rather than a major one: a surface looks it up and renders the spelling
// itself where it finds no word for it. That is the same argument
// `FleetCapacity.held_by` is made on in `docs/practices/protocol.md`.
//
// `familyOf` reads one prefix, and its own note says why that is not a branch
// on a value the backend chose: `status_<destination>` is composed here.

import type {
  EvidenceSubmitted,
  JobDetail,
  JobFilesChanged,
  JobHistory,
  Recorded,
  StepAttempt,
  StepDetail,
  Submitted,
} from "@armada/protocol";

import { CHECK_ADVANCES, CHECK_OUTCOME, JOB_LIFECYCLE, JOB_STATUS } from "@armada/components";
import { fileNameOf } from "../file-name";
import { repositorySaid } from "../gate-policy";
import { caseRunsOf } from "./cases";
import { coordOfStep, type RunCoord } from "./coord";
import type { GroupView } from "./group";
import { taskViewsOf, type TaskView } from "./task";

/**
 * Who a row is about. The wire's `Actor`, plus the three the new shape needs.
 *
 * `contributor` is somebody who is not you, on their own machine. **Nothing
 * derives one today** — it needs the store between Armada instances that
 * `#1530` files separately — so the value exists for the shape's sake and the
 * derivation below never mints it.
 */
export type LedgerActor =
  | "person"
  | "contributor"
  | "fleet"
  | "drone"
  | "judge"
  | "check";

/**
 * One row of the Record.
 *
 * **`coord` and `actor` replace `step` and `task?`** (#1532). A row inside a
 * group could not be placed by a step id alone, and a row about no step at all
 * — the Job's own machine moving — needs to say so rather than carry a blank.
 */
export type LedgerRow = {
  at: string;
  /**
   * Where it happened. **`null` is a fact about the Job and not about any
   * step** — its own machine moving, a person approving it. Not a gap.
   */
  coord: RunCoord | null;
  actor: LedgerActor;
  /** An opaque string. A surface renders the spelling where it has no word. */
  kind: string;
  /** What happened, in words. */
  what: string;
  /** What it came to, in words. Empty where the row states no outcome. */
  outcome: string;
  /**
   * What a reader asks for the next page with.
   *
   * **The log's own `seq`, which is monotonic and never reused** — and never
   * `at`, which is injected rather than read from a clock, so two rows inside
   * one millisecond carry the same instant.
   */
  cursor: number;
};

/**
 * The eight the Record's filters divide rows into, beside All.
 *
 * **Seven name a producer and `job` names the Job itself** — the owner asked
 * for a Job filter on 28 September 2026, standing on the very line that said
 * the Job's own moves answered to nothing. **No row is ever counted twice**,
 * which is what `#1537`'s own finding is about: a board whose All said 34 while
 * its filters summed to 35. It is still not that the eight sum to All, because
 * `kind` is opaque — see `familyOf`.
 */
export type LedgerFamily =
  | "job"
  | "evidence"
  | "files"
  | "checks"
  | "judges"
  | "drones"
  | "tasks"
  | "tests";

/** The eight, in the order the filters draw them. Job first, as All's nearest. */
export const LEDGER_FAMILIES: readonly LedgerFamily[] = [
  "job",
  "evidence",
  "files",
  "checks",
  "judges",
  "drones",
  "tasks",
  "tests",
];

// The spellings each family answers to. `kind` stays opaque on the wire (the
// header says why), so this is a lookup a surface does and never a branch the
// type forces. A kind that is not here belongs to no family, which `familyOf`
// answers with rather than guessing.
const FAMILY_OF: Readonly<Record<string, LedgerFamily>> = {
  created: "job",
  started: "job",
  evidence_submitted: "evidence",
  handed_in: "evidence",
  deliverable_kept: "evidence",
  file_written: "files",
  task_files: "files",
  checked: "checks",
  judged: "judges",
  flagged: "judges",
  drone_spawned: "drones",
  drone_exited: "drones",
  plan_recorded: "tasks",
  plan_revised: "tasks",
  task_open: "tasks",
  task_working: "tasks",
  task_handed_in: "tasks",
  task_done: "tasks",
  task_failed: "tasks",
  task_dropped: "tasks",
  touched_after_done: "tasks",
  case_run: "tests",
  cases_rerun: "tests",
  shown_again: "tests",
  frames_kept: "tests",
};

/**
 * Which filter a row answers to, or `null` for a row that answers to none.
 *
 * **The Job's own machine moving is `job` and never a Task.** Creating a Job,
 * approving it and merging its pull request produce no evidence, no file, no
 * Check, no Judge, no Drone, no task and no case run, so they have a filter of
 * their own rather than being folded into one that would then lie.
 *
 * **A kind this module has never heard of is still `null`.** The invariant is
 * that no row is counted twice, not that the eight sum to All — a surface that
 * draws the counts owes a reader the difference, see `unfiledIn`.
 */
export function familyOf(kind: string): LedgerFamily | null {
  // `status_<destination>` is composed above, in `kindOf` and `jobRowsOf`, out
  // of a `Movement` the wire spells as a kind and a destination — so reading
  // the prefix back is reading this module's own spelling, not branching on a
  // value the backend chose.
  if (kind.startsWith("status_")) return "job";
  return FAMILY_OF[kind] ?? null;
}

/** How many rows each filter holds. Never more than `rows.length` in total. */
export function countsOf(rows: readonly LedgerRow[]): Record<LedgerFamily, number> {
  const counts = {
    job: 0,
    evidence: 0,
    files: 0,
    checks: 0,
    judges: 0,
    drones: 0,
    tasks: 0,
    tests: 0,
  };
  for (const row of rows) {
    const family = familyOf(row.kind);
    if (family !== null) counts[family] += 1;
  }
  return counts;
}

/**
 * The rows no filter holds — under All and nowhere else.
 *
 * **What a reader would otherwise have to work out from the strip.** All says
 * 34 and the families sum to 30; these are the four, and a surface says so
 * rather than leaving the subtraction to whoever noticed.
 */
export function unfiledIn(rows: readonly LedgerRow[]): LedgerRow[] {
  return rows.filter((row) => familyOf(row.kind) === null);
}

/** Every row of a Job's Record, oldest first. */
export function ledgerRowsOf(history: JobHistory): LedgerRow[] {
  return history.moves.map(ledgerRowOf);
}

/** One recorded move, as a Record row. */
export function ledgerRowOf(move: Recorded): LedgerRow {
  return {
    at: move.at,
    coord: coordOf(move),
    actor: actorOf(move.actor),
    kind: kindOf(move),
    what: whatOf(move),
    outcome: outcomeOf(move),
    cursor: move.seq,
  };
}

// The Job's own machine moving names no step, so it is `null`. A step move and
// a Drone arriving both name one, and neither names a group or a task — the
// wire has neither, so the coordinate stops at the step.
function coordOf(move: Recorded): RunCoord | null {
  if (move.moved.kind === "status") {
    return null;
  }
  return { step: move.moved.step_id, step_attempt: 1 };
}

// `movement_kind` is the registry's own vocabulary for the three shapes. A
// status move is qualified by where it went, and a Drone move by its
// `presence`, so one Record can hold `drone_spawned` and `drone_exited` as
// different rows rather than as one row a reader has to open.
function kindOf(move: Recorded): string {
  switch (move.moved.kind) {
    case "status":
      return `status_${move.moved.to}`;
    case "step":
      return "step";
    case "drone":
      return move.moved.presence;
  }
}

// **Every `what` leads with its kind and what happened, then ` · ` and what it
// happened to** (the owner, 29 Sep 2026: *I dont understand what these rows
// are representing*). A Check's name and a file's path are the exceptions, and
// neither is composed here.
function whatOf(move: Recorded): string {
  switch (move.moved.kind) {
    case "status":
      return jobMovedSays(move.status, move.moved.to);
    case "step":
      return `Step moved · ${move.moved.step_id}, ${move.moved.from} to ${move.moved.to}`;
    case "drone":
      return `${DRONE_SAYS[move.moved.presence] ?? "Drone moved"} · ${move.moved.drone_id} on ${move.moved.step_id}`;
  }
}

/** A Drone arriving and leaving, as a row's lead. Sentence case. */
const DRONE_SAYS: Readonly<Record<string, string>> = {
  drone_spawned: "Drone started",
  drone_exited: "Drone ended",
};

/**
 * The Job's own status moving, as a row's `what`. **A move to a terminal status
 * is the Job ending**, and says what it ended as; any other move names both
 * ends, in the registry's own words.
 */
function jobMovedSays(from: string, to: string): string {
  if (JOB_LIFECYCLE[to]?.terminal === true) return `Job ended · ${statusSaid(to)}`;
  return `Job moved · ${statusSaid(from)} to ${statusSaid(to)}`;
}

function statusSaid(status: string): string {
  return JOB_STATUS[status]?.verb ?? status.replaceAll("_", " ");
}

// The reason a row carries, where it carries one. A status move's `reason` is
// absent on the eight destinations that store none, and a step move's `why` is
// present on the one move that stops it — so an empty outcome is the ordinary
// case rather than a value that failed to load.
function outcomeOf(move: Recorded): string {
  switch (move.moved.kind) {
    case "status":
      return move.moved.reason?.named ?? "";
    case "step":
      return move.moved.why ?? "";
    case "drone":
      return "";
  }
}

// The wire's `Actor` gained `judge` and `check` at 22.0: a Judge's refusal and
// a failed Check sign the rows their answer wrote. `helm` and anything this
// Bridge has never heard of read as `fleet` rather than as a guess.
function actorOf(actor: string): LedgerActor {
  switch (actor) {
    case "drone":
      return "drone";
    case "human":
      return "person";
    case "judge":
      return "judge";
    case "check":
      return "check";
    default:
      return "fleet";
  }
}

// ------------------------------------------------- the whole Record, composed
//
// **`GET /jobs/:job_id/events` is three kinds of row and the Record draws
// seven.** A Check run, a Judge verdict, a file written, an evidence claim, a
// task finishing and a case run are none of them a `Movement`, and every one of
// them is already on a read Bridge takes to draw a Job. So the ledger is
// composed here from those reads, and the server-side ledger read replaces this
// wholesale when the backend lands (`#1537`).
//
// **Where the history was read, it owns the moves it carries** — the Job's own
// machine, the steps and the Drones — and the detail supplies the rest. Where
// it was not, those rows are derived from the detail instead, so a Job opened
// before its history answers still draws a Record rather than a blank.

/** What the Record is composed from. Every field but `detail` is optional. */
export type LedgerReads = {
  /** `GET /jobs/:job_id`, for the Job on screen. */
  detail: JobDetail;
  /** `GET /jobs/:job_id/events`, where it was read. */
  history?: readonly Recorded[];
  /** `GET /jobs/:job_id/evidence`, where it was read. */
  evidence?: readonly Submitted[];
  /** The last `job.files_changed` heard for this Job. */
  footprint?: JobFilesChanged;
  /** The last `evidence.submitted` heard for this Job. */
  handed?: EvidenceSubmitted;
  /**
   * The plan's groups, where the draft holds them. **A task's own facts live
   * here and not on the wire** — why it failed, the cases it owes, whether a
   * later task edited it — so its row reads them where they exist.
   */
  groups?: readonly GroupView[];
};

/**
 * One Job's Record, **newest first**.
 *
 * `at` orders it and `cursor` breaks a tie, which is the inverse of the wire's
 * rule — the wire's `seq` exists for every row it carries, and most rows here
 * are derived from a read that has no `seq` at all.
 */
export function ledgerOf(reads: LedgerReads): LedgerRow[] {
  const { detail } = reads;
  const moves = reads.history ?? [];
  const rows: LedgerRow[] = moves.map((move) => withPolicy(ledgerRowOf(move), move, detail));
  const mint = minting(moves);

  if (moves.length === 0) rows.push(...jobRowsOf(detail, mint));
  for (const step of detail.steps) {
    rows.push(...checkRowsOf(detail, step, mint));
    rows.push(...judgeRowsOf(detail, step, mint));
    rows.push(...evidenceKeptOf(step, mint));
    if (moves.length === 0) rows.push(...droneRowsOf(step, mint));
  }
  rows.push(...planRowsOf(detail, mint));
  rows.push(...taskRowsOf(detail, reads.groups, mint));
  rows.push(...fileRowsOf(detail, reads.footprint, mint));
  rows.push(...handedRowsOf(detail, reads.evidence, reads.handed, mint));
  rows.push(...testRowsOf(detail, mint));

  return rows.sort(newestFirst);
}

/**
 * A step move that closed a run, with what the repository said where its gate
 * asked it. #1683.
 *
 * **Joined on the instant, which is exact.** `StepAttempt::over` closes a run
 * at the `at` of the move that left `running`, so the run a move ended is the
 * one whose `ended_at` is that move's own.
 */
function withPolicy(row: LedgerRow, move: Recorded, detail: JobDetail): LedgerRow {
  if (move.moved.kind !== "step" || move.moved.from !== "running") return row;
  const stepId = move.moved.step_id;
  const step = detail.steps.find((one) => one.step_id === stepId);
  const run = step?.attempts.find((one) => one.ended_at === move.at);
  const said = step === undefined ? undefined : repositorySaid(step, run);
  if (said === undefined) return row;
  return { ...row, outcome: row.outcome === "" ? sentenceCase(said) : `${row.outcome}: ${said}` };
}

// A cursor for a row the log never carried. It starts past every `seq` the
// history holds, so a derived row can never collide with a recorded one, and it
// climbs in the order the rows are built — which is what settles two rows that
// share an instant.
function minting(moves: readonly Recorded[]): () => number {
  let next = moves.reduce((highest, move) => Math.max(highest, move.seq), 0) + 1;
  return () => next++;
}

function newestFirst(left: LedgerRow, right: LedgerRow): number {
  if (left.at === right.at) return right.cursor - left.cursor;
  return left.at < right.at ? 1 : -1;
}

// ---------------------------------------------------------------- the Job
//
// **Only where no history was read.** Creation is not a transition and the log
// carries no row for it (`JobHistory.moves` says so), so it is derived either
// way — but the approval and the end are status moves the log does carry, and
// deriving them beside the log's own would be the same fact twice.

function jobRowsOf(detail: JobDetail, mint: () => number): LedgerRow[] {
  const job = detail.job;
  const rows: LedgerRow[] = [
    {
      at: detail.created_at,
      coord: null,
      actor: job.origin === "manual" ? "person" : "fleet",
      kind: "created",
      what: "Job created",
      outcome: "",
      cursor: mint(),
    },
  ];
  // `started_at` is the first arrival at `running` and nothing else
  // (`JobSummary`), so a Job still at its gate has not reached it and draws no
  // such row rather than one dated from a field that means something else.
  if (job.started_at !== undefined && job.status !== "awaiting_approval" && job.status !== "queued") {
    rows.push({
      at: job.started_at,
      coord: null,
      actor: "fleet",
      kind: "started",
      what: "Job started",
      outcome: "",
      cursor: mint(),
    });
  }
  if (job.ended_at !== undefined) {
    rows.push({
      at: job.ended_at,
      coord: null,
      actor: "fleet",
      kind: `status_${job.status}`,
      what: `Job ended · ${statusSaid(job.status)}`,
      outcome: job.landed === undefined ? "" : `The pull request ${job.landed}`,
      cursor: mint(),
    });
  }
  return rows;
}

// -------------------------------------------------------------- the gates

/**
 * **A Check run is its own row, and `check` is who ran it.** Today's wire says
 * Fleet caused everything a gate does, so nothing could derive this actor from
 * an `Actor` field — it is read off which list the row came out of instead,
 * which is the only thing that separates a Check from a Judge on this seam.
 *
 * **The outcome is the registry's word alone** — `Passed`, `Failed` (the owner,
 * 29 Sep 2026: *I can see details when I open the row*). What the run produced
 * is the row's sheet, which reads it off the `CheckRun` itself. **One addition,
 * the owner's of 2 Oct 2026**: a red run on a test another Job is already
 * fixing names that Job after the word, since it changes what a person does.
 */
function checkRowsOf(detail: JobDetail, step: StepDetail, mint: () => number): LedgerRow[] {
  const latest = step.check_runs.reduce((at, run) => Math.max(at, run.attempt), 0);
  return step.check_runs.map((run) => {
    const verb = sentenceCase(CHECK_OUTCOME[run.outcome]?.verb ?? run.outcome.replaceAll("_", " "));
    const fixing = run.attempt === latest && CHECK_ADVANCES[run.outcome] === false
      ? fixingIt(detail, run.name)
      : undefined;
    return {
      at: atOf(step, run.attempt),
      coord: {
        step: step.step_id,
        step_attempt: run.attempt,
        // The group whose gate ran it, where Fleet records one (#1652).
        ...(run.group === undefined ? {} : { group: run.group }),
        ...(run.group_attempt === undefined ? {} : { group_attempt: run.group_attempt }),
      },
      actor: "check" as const,
      kind: "checked",
      what: run.name,
      outcome: fixing === undefined ? verb : `${verb} — ${fixing}`,
      cursor: mint(),
    };
  });
}

/**
 * The Job already fixing the test a Check failed on, where it is another Job —
 * matched by the Check's name, as Overview's lead matches it. #1673.
 *
 * **On the latest attempt's red run alone**, the one the lead names: an
 * earlier run of the same Check may have failed for another reason, and the
 * claim says nothing about which attempt hit the test.
 */
function fixingIt(detail: JobDetail, check: string): string | undefined {
  const claim = (detail.breakages ?? []).find((one) => one.check === check && one.fix !== detail.job.id);
  return claim === undefined ? undefined : `${claim.fix_title} is already fixing it`;
}

function sentenceCase(said: string): string {
  return said.charAt(0).toUpperCase() + said.slice(1);
}

/** A criterion the Judge answered, and a pattern it flagged. `judge` ran both. */
function judgeRowsOf(detail: JobDetail, step: StepDetail, mint: () => number): LedgerRow[] {
  const rows: LedgerRow[] = step.judged.map((answer) => ({
    at: atOf(step, answer.attempt),
    coord: { step: step.step_id, step_attempt: answer.attempt },
    actor: "judge" as const,
    kind: "judged",
    what: criterionSays(detail, answer.criterion_id),
    outcome: sentenceCase(
      answer.produced === undefined ? said(answer.verdict) : `${said(answer.verdict)} — ${answer.produced}`,
    ),
    cursor: mint(),
  }));
  for (const flag of step.flagged) {
    rows.push({
      at: atOf(step, flag.attempt),
      coord: { step: step.step_id, step_attempt: flag.attempt },
      actor: "judge",
      kind: "flagged",
      what: `Pattern flagged · ${flag.pattern}`,
      outcome: flag.cited,
      cursor: mint(),
    });
  }
  return rows;
}

/**
 * A criterion the Judge answered, by its frozen position and its words: `Criterion 2
 * judged · …`. The verdict is the row's outcome and never repeated here.
 */
function criterionSays(detail: JobDetail, criterionId: string): string {
  const at = detail.acceptance_criteria.findIndex((one) => one.criterion_id === criterionId);
  const criterion = detail.acceptance_criteria[at];
  return criterion === undefined
    ? `Criterion judged · ${criterionId}`
    : `Criterion ${at + 1} judged · ${criterion.text}`;
}

// `criterion_verdict_judge` is `met` or `not_met`, and the underscore is the
// wire's spelling rather than a word. The generated vocabulary has no entry for
// it, so the one place it becomes English is here.
function said(verdict: string): string {
  return verdict === "not_met" ? "not met" : verdict;
}

// -------------------------------------------------------------- the Drones
//
// **One row per Drone per task, never one Drone for a whole step** (`#1530`).
// Today's wire holds one Drone per step attempt and no per-task Drone at all —
// `TaskView.drone_id` is the draft field that will carry it — so a step's own
// attempt is what a row is built from, and the row names the step it ran.
// It never collapses several Drones into one row, because there are not several
// to collapse yet.

function droneRowsOf(step: StepDetail, mint: () => number): LedgerRow[] {
  const rows: LedgerRow[] = [];
  for (const attempt of step.attempts) {
    rows.push({
      at: attempt.started_at,
      coord: { step: step.step_id, step_attempt: attempt.attempt },
      actor: "drone",
      kind: "drone_spawned",
      what: `Drone started · ${step.label}`,
      outcome: "",
      cursor: mint(),
    });
    if (attempt.ended_at === undefined) continue;
    rows.push({
      at: attempt.ended_at,
      coord: { step: step.step_id, step_attempt: attempt.attempt },
      actor: "drone",
      kind: "drone_exited",
      what: `Drone ended · ${step.label}`,
      outcome: sentenceCase(ranTo(step, attempt)),
      cursor: mint(),
    });
  }
  return rows;
}

/** What a run came to, and what the repository said where its gate asked it. */
function ranTo(step: StepDetail, attempt: StepAttempt): string {
  const outcome = attempt.why === undefined ? attempt.outcome : `${attempt.outcome} — ${attempt.why}`;
  const said = repositorySaid(step, attempt);
  return said === undefined ? outcome : `${outcome}: ${said}`;
}

// ---------------------------------------------------------- the plan's work

/** The plan being written down. **Fleet where a step recorded it**, a person where one did. */
function planRowsOf(detail: JobDetail, mint: () => number): LedgerRow[] {
  const plan = detail.work_plan;
  if (plan === undefined) return [];
  const by = plan.recorded_by;
  const tasks = plan.tasks.length;
  return [
    {
      at: plan.recorded_at,
      coord: by.by === "step" ? { step: by.step_id, step_attempt: by.attempt } : null,
      actor: by.by === "person" ? "person" : "fleet",
      kind: "plan_recorded",
      // The approach is a paragraph and belongs to the Plan tab. What this row
      // owes a reader is how much work came out of it — said once, in What.
      what: `Plan recorded · ${tasks} ${tasks === 1 ? "task" : "tasks"}`,
      outcome: "",
      cursor: mint(),
    },
  ];
}

/**
 * What was done to each task, and why.
 *
 * **A row is an action on the task** (the owner, 29 Sep 2026: *if the task was
 * marked done that should be what the action was*): `T5 marked done`, `T6
 * marked failed`, `T3 dropped`, `T6 started`. The outcome is the short reason —
 * what it showed, why it failed, why it was dropped — and the row's sheet reads
 * the rest off the task.
 *
 * **A task's own "done" is never Evidence** (`#1530`, 21 Sep). **The draft's
 * tasks where it holds them**: only the draft knows a task failed, or that a
 * later task edited one that was done. **The instant is the step's run's**,
 * since no field on the wire stamps a task.
 */
function taskRowsOf(
  detail: JobDetail,
  groups: readonly GroupView[] | undefined,
  mint: () => number,
): LedgerRow[] {
  const rows: LedgerRow[] = [];
  const tasks = groups === undefined ? taskViewsOf(detail) : groups.flatMap((group) => group.tasks);
  for (const task of tasks) {
    if (task.state === "open") continue;
    const step = detail.steps.find((one) => one.step_id === task.coord.step);
    const at = step === undefined ? detail.created_at : atOf(step, task.coord.step_attempt);
    rows.push({
      at,
      coord: task.coord,
      actor: task.state === "dropped" ? "person" : "drone",
      kind: `task_${task.state}`,
      what: `${task.id} ${TASK_ACTION[task.state]} · ${task.title}`,
      outcome: reasonOf(task),
      cursor: mint(),
    });
    if (!task.touched_after_done) continue;
    const later = laterTaskOf(task, tasks);
    rows.push({
      at,
      coord: task.coord,
      actor: "drone",
      kind: "touched_after_done",
      what: `${task.id} changed after done · ${task.title}`,
      outcome:
        later === undefined
          ? "A later task edited a file it had finished"
          : `${later.id} edited a file it had finished`,
      cursor: mint(),
    });
  }
  return rows;
}

/** What was done to a task, by the state it moved to. */
const TASK_ACTION: Record<Exclude<TaskView["state"], "open">, string> = {
  working: "started",
  handed_in: "handed in",
  done: "marked done",
  failed: "marked failed",
  dropped: "dropped",
};

/** The short reason a task's row gives: what it showed, or why it stopped. */
function reasonOf(task: TaskView): string {
  switch (task.state) {
    case "handed_in":
    case "done":
      return task.shown ?? "";
    case "failed":
      return task.failed_reason ?? "";
    case "dropped":
      return task.reason ?? "";
    default:
      return "";
  }
}

/**
 * The later task that edited a file this one had finished: the first after it,
 * in plan order, claiming a path it claimed. `tab-plan-read.ts`'s `touchedByOf`
 * reads the same join for the Plan board.
 */
function laterTaskOf(task: TaskView, tasks: readonly TaskView[]): TaskView | undefined {
  const claimed = new Set(task.scope);
  return tasks
    .slice(tasks.indexOf(task) + 1)
    .find((candidate) => candidate.scope.some((path) => claimed.has(path)));
}

// ------------------------------------------------------------------- files

/**
 * What was written, and whether anything said it would be.
 *
 * Two sources and they say different things: the footprint is what git found on
 * the branch, and a finished task's scope is what the planner said that task
 * would touch.
 *
 * **Neither row says "the plan".** `docs/concepts/plan.md`, *Distinct from the
 * declared scope's "plan"*, forbids the collision outright — a Job's Plan is
 * tasks and a step's declared scope is paths — and the owner asked what *inside
 * the plan* meant on a file row (28 September 2026) because the collision had
 * reached a person. So each row spells out who said what, and `outside_plan` is
 * *a mark, not a judgement* (`packages/protocol/src/events.ts`), which is why
 * neither takes a hue.
 */
function fileRowsOf(
  detail: JobDetail,
  footprint: JobFilesChanged | undefined,
  mint: () => number,
): LedgerRow[] {
  const rows: LedgerRow[] = [];
  if (footprint !== undefined) {
    for (const file of footprint.files) {
      rows.push({
        at: footprint.at,
        coord: { step: footprint.step_id, step_attempt: 1 },
        actor: "drone",
        kind: "file_written",
        what: file.path,
        outcome:
          file.outside_plan === true
            ? `${sentenceCase(file.change)}, out of scope`
            : sentenceCase(file.change),
        cursor: mint(),
      });
    }
  }
  const targets = detail.write_targets;
  for (const task of taskViewsOf(detail)) {
    if (task.state !== "done" || task.scope.length === 0) continue;
    const step = detail.steps.find((one) => one.step_id === task.coord.step);
    const outside = targets === undefined ? [] : task.scope.filter((path) => !within(path, targets));
    rows.push({
      at: step === undefined ? detail.created_at : atOf(step, task.coord.step_attempt),
      coord: task.coord,
      actor: "drone",
      kind: "task_files",
      what: task.scope.join(PATHS_JOINED),
      outcome: saidItWouldChange(targets, outside),
      cursor: mint(),
    });
  }
  return rows;
}

/** How a task's files are joined into one `what`, so `pathsOf` can part them. */
const PATHS_JOINED = ", ";

/**
 * The paths a file row names: one for `file_written`, the task's scope for
 * `task_files`. Empty for a row of any other kind.
 */
export function pathsOf(row: LedgerRow): string[] {
  if (row.kind === "file_written") return [row.what];
  if (row.kind === "task_files") return row.what.split(PATHS_JOINED);
  return [];
}

/**
 * What a finished task's files came to, against what the Job said it would
 * change: `In scope`, or the files that were not, by name.
 *
 * **The owner's own words** (29 Sep 2026), replacing sentences that spelled out
 * who said what and read as a riddle. A file goes by its name and never its
 * path, as the What cell's chips do. The sentence is the table's Outcome alone:
 * the row's sheet draws the file's diff in its place.
 */
function saidItWouldChange(
  targets: readonly string[] | undefined,
  outside: readonly string[],
): string {
  if (targets === undefined) return "this Job named no files it would change, so there is nothing to compare";
  if (outside.length === 0) return "In scope";
  const names = outside.map(fileNameOf);
  const listed =
    names.length === 1 ? names[0] : `${names.slice(0, -1).join(", ")} and ${names[names.length - 1]}`;
  return `${listed} ${names.length === 1 ? "was" : "were"} out of scope`;
}

// `write_targets` are prefixes — a directory, or a file. Absent is scope
// undetermined and present-and-empty is writing nothing, which is why the
// caller checks for absence rather than treating an empty list as "anywhere".
function within(path: string, targets: readonly string[]): boolean {
  return targets.some((target) => path === target || path.startsWith(target));
}

// ---------------------------------------------------------------- evidence

/** The copies of a deliverable Fleet kept, one row each. */
function evidenceKeptOf(step: StepDetail, mint: () => number): LedgerRow[] {
  return (step.deliverables ?? []).map((kept) => ({
    at: atOf(step, kept.attempt),
    coord: { step: step.step_id, step_attempt: kept.attempt },
    actor: "fleet" as const,
    kind: "deliverable_kept",
    what: kept.path,
    outcome: "",
    cursor: mint(),
  }));
}

/** What a Drone claimed, and the moment one handed in. */
function handedRowsOf(
  detail: JobDetail,
  evidence: readonly Submitted[] | undefined,
  handed: EvidenceSubmitted | undefined,
  mint: () => number,
): LedgerRow[] {
  const rows: LedgerRow[] = (evidence ?? []).map((one) => {
    const step = detail.steps.find((candidate) => candidate.step_id === one.step_id);
    return {
      at: step === undefined ? detail.created_at : atOf(step, lastAttemptOf(step)),
      coord: step === undefined ? null : coordOfStep(step),
      actor: "drone" as const,
      kind: "evidence_submitted",
      what: `Evidence submitted · ${one.claimed}`,
      outcome: one.shown_by,
      cursor: mint(),
    };
  });
  // The pushed moment, kept only where the asked-for read has not landed: the
  // two are the same submission, and drawing both would be one hand-in twice.
  if (handed !== undefined && rows.length === 0) {
    rows.push({
      at: handed.at,
      coord: { step: handed.step_id, step_attempt: 1 },
      actor: "drone",
      kind: "handed_in",
      what: `Evidence handed in · ${handed.step_id}`,
      outcome: handed.evidence_type,
      cursor: mint(),
    });
  }
  return rows;
}

// ------------------------------------------------------------------- tests

/**
 * Every run of a case. **A case run is never an Evidence row** (`#1530`), and
 * `familyOf` files these under Tests.
 *
 * **A run with no step reads as after the Job** — the coordinate carries the
 * `null` and the surface says the words.
 */
function testRowsOf(detail: JobDetail, mint: () => number): LedgerRow[] {
  return caseRunsOf(detail).map((run) => ({
    at: run.ran_at,
    coord: run.coord,
    actor: run.actor === "contributor" ? ("contributor" as const) : (run.actor as LedgerActor),
    kind: "case_run",
    what: `Case run · ${run.case}`,
    outcome: `${run.outcome === "ran" ? "Ran" : run.outcome === "run_failed" ? "The run failed" : "Not covered"}, ${run.frames} frames`,
    cursor: mint(),
  }));
}

// --------------------------------------------------------------- instants
//
// **A run of a step is the nearest instant the wire has for anything inside
// it.** A Check run, a Judge answer and a kept deliverable all carry which
// attempt produced them and none of them carries a clock, so each takes the end
// of that run — or its start, where the run is the one still going.

function attemptOf(step: StepDetail, attempt: number): StepAttempt | undefined {
  return step.attempts.find((one) => one.attempt === attempt);
}

function atOf(step: StepDetail, attempt: number): string {
  const run = attemptOf(step, attempt);
  // A run still going has no end, and its start is the wrong instant for a
  // Check that has already answered — every row inside a live step would stack
  // on the moment the Drone arrived. The step's own `updated_at` is the last
  // thing anything moved on it, which is the nearest upper bound there is.
  return run?.ended_at ?? step.updated_at ?? run?.started_at ?? step.entered_at;
}

function lastAttemptOf(step: StepDetail): number {
  return step.attempts.length === 0 ? 1 : step.attempts[step.attempts.length - 1]!.attempt;
}
