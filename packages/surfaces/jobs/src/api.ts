// Jobs: proposing, deciding, steering and reading one Job, its runs and its merge line.
// A slice imports protocol and screens, never another slice; `apps/desktop/src/shared/api/jobs.ts` re-exports it.

import type {
  AddTask,
  FixMain,
  EditTask,
  MovePlan,
  ApproveWave,
  EditJob,
  ToProposer,
  ApproveDispatch,
  BranchesRead,
  Artifact,
  CheckOutputRead,
  BriefRead,
  LessonAnswer,
  LessonsRead,
  RetroRead,
  AlwaysAllowScope,
  CommandAnswer,
  CommandExplainedRead,
  DropTask,
  Followed,
  FrameRead,
  JudgeAnswer,
  Opened,
  Outcome,
  Proposed,
  RunListRead,
  RunOutputRead,
  StagedAttachment,
  StartRun,
  WhenBlocked,
  WhenRefused,
  LandCheckAt,
  Diff,
  Evidence,
  Examination,
  Footprint,
  Handed,
  Crewed,
  Holds,
  History,
  FollowedLog,
  Journalled,
  Observed,
  Remarks,
  Watched,
  MergeLines,
  ProposalInFlight,
  FollowedLandLog,
  RunFollowed,
  RunSheetRead,
} from "@armada/protocol";
import type { PlanEditAnswer } from "@armada/screens/src/plan-edits";
import type { ComposingRead } from "@armada/screens/src/composing-reads";

export type JobsApi = {
  /**
   * Describe the work and let the Job proposer decide what it is: which
   * workflow, what to call it, and whether it is one Job or several.
   *
   * **The only way this window makes a Job.** Every one comes back at
   * `awaiting_approval` and each takes its own approval in turn — approving
   * one of several accepts a plan and starts nothing else. `proposeJob` was
   * the hand form's entry and went with it on 2026-09-23; the workflow a
   * person wants to name themselves is a Settings field on the same card.
   *
   * The two refusals are separate arms of `Proposed` because a person does
   * different things about them — a request nothing fits is said again
   * differently, and a call that could not be made is simply asked again.
   *
   * `repository` is the root New job's ask answered on All. The Board stays
   * on All while composing (#959), so the request names what was answered
   * rather than a pick that never moved. `null`, the default, where a
   * repository was already picked before the composer opened.
   */
  proposeFromRequest: (
    request: string,
    attachments: readonly StagedAttachment[],
    repository?: string | null,
  ) => Promise<Proposed>;
  /**
   * Stop the proposal this window is waiting on.
   *
   * **It kills the call rather than stopping the wait.** A window that merely
   * dropped the request would leave the proposer running inside Fleet, spending
   * against the budget, with nobody left to read what it decided.
   *
   * Takes no id: `BridgeState.proposing` is what this window is waiting on, and
   * an id from the renderer would let one window stop another's call. Answers
   * whether there was still one to stop — pressing this a beat after the Jobs
   * landed is being late rather than failing, and the surface says so.
   */
  stopProposal: () => Promise<Outcome>;
  /**
   * Write pasted or picked bytes to a staging file before a Job exists —
   * there is no Job id yet to key storage on; one is minted at `propose`
   * time. Returns the absolute path written, which the caller carries until
   * `proposeFromRequest` sends it as a `staged_path`.
   */
  stageAttachment: (
    bytes: ArrayBuffer,
    filename: string,
    mimeType: string,
  ) => Promise<{ path: string }>;
  /**
   * Paths under the checkout narrowed against typed text, for the `@` mention
   * popup. **Empty rather than a fault**, on a call that could not be made or
   * on nothing connected — a person is typing, and a toast over a popup they
   * may not even have open would be Bridge announcing a failure nobody asked
   * to hear about.
   */
  searchFiles: (query: string) => Promise<string[]>;
  /**
   * Release a Job at its approval gate, with the proposal as the person left
   * it (`approve_dispatch`, since 23.8). **Absent is the proposal as it
   * stands**, which is Helm's press and every approval nobody edited.
   */
  approveDispatch: (jobId: string, approval?: ApproveDispatch) => Promise<Outcome>;
  /**
   * The repository's branches, the base first (`list_branches`, #1605), for
   * the two branch fields to offer.
   */
  listBranches: (manifestId: string) => Promise<BranchesRead>;
  /**
   * Kill the failed Job and mint its replacement. **Nothing resumes** — the
   * Job it is called on ends at `killed` and a new one is created carrying
   * `redispatched_from`, whose id comes back on the outcome.
   */
  redispatchJob: (jobId: string) => Promise<Outcome>;
  /**
   * Kill the process. The Job survives, with its worktree held. **`droneId`
   * names one Drone of several** (`kill_one_drone`, 23.10): the others go on,
   * and one that is not live is refused as `fleet.drone_not_live`. Absent is
   * the Job's kept Drone and every one beside it.
   */
  killDrone: (jobId: string, droneId?: string) => Promise<Outcome>;
  /** End the Job at `killed`. Terminal, and nothing resumes it. */
  killJob: (jobId: string) => Promise<Outcome>;
  /** Pause the Job and keep it: its work is parked on its branch and its slot goes back. */
  parkJob: (jobId: string) => Promise<Outcome>;
  /** Lift the pause. A gate Job takes a slot now or waits for the first to free. */
  resumeJob: (jobId: string) => Promise<Outcome>;
  /** Hand a repository's red main to a Job: a new one, or an earlier one the work goes back to. */
  fixMain: (fix: FixMain) => Promise<Outcome>;
  /**
   * Kill one process the Job holds, by pid. **Fleet decides whether the pid is
   * the Job's** — it rebuilds the tree at the act — so this names, never grants.
   * A pid outside it is refused as `fleet.not_the_jobs_process`. #1647.
   */
  killProcess: (jobId: string, pid: number) => Promise<Outcome>;
  /** Kill every process the Job holds. #1647, as `killProcess`. */
  killProcesses: (jobId: string) => Promise<Outcome>;
  /**
   * Take the controls of a failed plan task's Drone. #250 builds the route;
   * until it does, the answer is `bridge.not_implemented`.
   */
  pilotTask: (jobId: string, taskId: string) => Promise<Outcome>;
  /** Run a failed plan task again. #1656, served since 23.4. */
  restartTask: (jobId: string, taskId: string) => Promise<Outcome>;
  /**
   * Change a plan task that is open or failed — only the fields `edit`
   * names. #1657, as `pilotTask`; the fields ride on the debug info.
   */
  editTask: (jobId: string, taskId: string, edit: EditTask) => Promise<Outcome>;
  /**
   * Move a group, or a task into a group, where a person dropped it on the
   * plan, by the task or group it now follows. #1685, served since 23.4.
   */
  movePlan: (jobId: string, move: MovePlan) => Promise<Outcome>;
  /**
   * Approve an Epic Job's plan and release every Job of the wave it proposed,
   * in one act. #1694, as `movePlan`; the Jobs ride on the debug info.
   */
  approveWave: (jobId: string, wave: ApproveWave) => Promise<Outcome>;
  /**
   * Save a Job's title, request or criteria while it waits at
   * `awaiting_approval`, without releasing it — `edit_job`, since 23.8. Only
   * the fields a person changed.
   */
  editJob: (jobId: string, edit: EditJob) => Promise<Outcome>;
  /** Where an approved Job with no landing target lands, once — `set_landing_target`, since 23.22. */
  setLandingTarget: (jobId: string, target: string) => Promise<Outcome>;
  /** Send the proposal back to the proposer with a note; the Job returns to its gate rewritten — `to_proposer`, since 23.25. */
  toProposer: (jobId: string, body: ToProposer) => Promise<Outcome>;
  /**
   * Inject an instruction into the Drone that is there. **Legal only on an
   * escalated Job that still holds one** — Fleet refuses 409 where the Drone
   * is gone, naming `restartStep` as the act that applies. Nothing is
   * spawned; the Job comes back `running` with the same session.
   */
  redirectDrone: (jobId: string, instruction: string, droneId?: string) => Promise<Outcome>;
  /**
   * Answer the question the job's drone asked, by picking one of the labels it
   * offered.
   *
   * **The answer is a choice, never prose.** There is no free-text parameter
   * and fleet refuses a label it did not offer — a person who needs to say
   * something the options do not cover uses `redirectDrone`, which is the one
   * route their own words reach a drone by.
   *
   * The job comes back unchanged: it was `running` while it waited and is
   * `running` now. Fleet refuses 409 where nothing is waiting, where the id
   * names a question already answered, and where the label was not offered.
   */
  answerQuestion: (
    jobId: string,
    questionId: string,
    chose: string,
  ) => Promise<Outcome>;
  /**
   * Allow or reject a command the job's drone reached for and was not given,
   * by the call id and one of the answers Fleet offered for it.
   *
   * **The call id says which of the two places it is.** A command a drone is
   * waiting on is answered in place and the job stays `running`; a refused row
   * on a job stopped at `blocked_by_policy` moves the job on. Fleet refuses 409
   * where the call names nothing waiting or refused, and where the answer was
   * not offered.
   *
   * **`rule` rides only with `always_allow`**, one of that command's own
   * candidates — a name outside them is a 409 too. **`scope` is read by an
   * `always_allow` alone** and is `kit` for Always allow on this machine, which
   * is sent with a rule; absent is the repository.
   */
  answerCommand: (
    jobId: string,
    call: string,
    answer: CommandAnswer,
    note?: string,
    rule?: string,
    scope?: AlwaysAllowScope,
  ) => Promise<Outcome>;
  /**
   * What one command does, in a cheap model's words, for the person deciding
   * whether to allow it.
   *
   * **A read, and it decides nothing.** The offers the command carries are
   * unchanged and the three answers stay live while this is out — a person who
   * never asks is answered exactly as before. It names the model, because a
   * reading is a claim rather than a fact.
   */
  explainCommand: (jobId: string, callId: string) => Promise<CommandExplainedRead>;
  /**
   * Change how one job meets a command its drone was not given. **Live**: the
   * next command the drone reaches for reads it, and no drone is respawned.
   */
  setWhenBlocked: (jobId: string, whenBlocked: WhenBlocked) => Promise<Outcome>;
  /**
   * Answer the question a judge refusal opened. One press is the whole
   * answer; `note` is never required. Fleet refuses 409 where the job is not
   * holding a question open.
   */
  answerJudge: (jobId: string, askedAt: string, answer: JudgeAnswer, note?: string) => Promise<Outcome>;
  /**
   * Change how one job meets a judge criterion that refuses. **Live**: the
   * next criterion that refuses reads it, and no drone is respawned.
   */
  setWhenRefused: (jobId: string, whenRefused: WhenRefused) => Promise<Outcome>;
  /**
   * Choose the model one job's later steps start on, or `null` for the one its
   * workflow gives each. **Live**: the step running now keeps its model.
   */
  setModel: (jobId: string, model: string | null) => Promise<Outcome>;
  /** Choose the model a job's review step starts on, or clear it. #903. */
  setReviewModel: (jobId: string, model: string | null) => Promise<Outcome>;
  /**
   * Take back a command a person allowed for one job, by the command exactly as
   * it was allowed. A line in `armada.yml` is not touched.
   */
  removeAllowedCommand: (jobId: string, run: string) => Promise<Outcome>;
  /**
   * Put a fresh Drone on the surviving worktree, at the step that stopped, and
   * say what to do differently where there is something to say.
   * **Legal only where the Drone is gone** — Fleet refuses 409 where one is
   * still alive, or where the worktree itself is gone.
   *
   * **The note is optional and the plain restart sends no body at all.** It
   * does not reach a session — there is none — it waits on the job and opens
   * the brief of the drone this asks for, which is where `requestChanges`'s
   * note goes. A blank one is not sent: `undefined` and `""` are both a
   * restart with nothing said, because a drone handed an empty instruction
   * starts over with exactly what was not enough.
   *
   * Fleet answers 409 where a note is already waiting on the job, quoting it
   * back rather than overwriting it.
   */
  restartStep: (jobId: string, note?: string) => Promise<Outcome>;
  /**
   * Overrule a machine that stopped the work, and let the Job go on.
   *
   * **Not an approval, and its own entry for that reason.** `approveReview`
   * answers a gate nothing objected to; this answers one that stopped the step,
   * and it says a machine was wrong rather than that the work was right. The
   * step advances still carrying `failed`, and the reason is written to the
   * Job's log where it stays.
   *
   * Legal on an escalated Job whose step stopped on `gate_failure` — the Judge
   * refusing a criterion — or on `evidence_suspect`, the gaming check calling
   * the evidence untrustworthy. Both are a machine's decision, which is what a
   * person may overrule, and on `gate_undecided`, where the Judge did not answer
   * and the person accepts the step themselves (the owner's call, 5 Oct 2026).
   * Fleet refuses 409 for a step that stopped on anything else; 422 for a
   * blank reason except on `evidence_suspect` and `gate_undecided`. Whether the Drone is still there
   * decides only how the Job carries on.
   */
  overrideVerdict: (jobId: string, reason: string) => Promise<Outcome>;
  /**
   * Ask the judge again, on the evidence the step already submitted.
   *
   * **Not an override.** `overrideVerdict` advances the step; `gate_undecided`
   * is a gate that made no decision — it could not derive what it needed to
   * read — so this asks the question that failed to be asked and moves nothing
   * by itself, which is why it carries no reason: nothing is being disputed, so there is
   * no sentence to record that the second reading will not say for itself.
   *
   * Legal on an escalated Job whose stopped step carries `gate_undecided`.
   * Fleet refuses 409 on any other trigger — that one is an override or
   * nothing — and on a Job it is no longer standing at, because the baseline
   * the gate reads against lives in that Job's slot and a Fleet restarted since
   * the escalation has none. Fleet stands at several Jobs at once, so the
   * refusal is about this Job's slot being gone rather than about the one slot
   * holding somebody else. Where the cause has not gone away the gate
   * is undecided again and **nothing moves**, which is not a failure.
   */
  rerunGate: (jobId: string) => Promise<Outcome>;
  /**
   * Run a stopped step's Checks again, on an `awaiting_repair` Job whose
   * stopped step failed a mechanical Check. No Drone spawns, no retry is
   * spent — `#1105`. **This can take minutes**, since it waits on the Checks
   * themselves, not a store read.
   */
  rerunChecks: (jobId: string) => Promise<Outcome>;
  /**
   * Ask a Job to show its work: Fleet reruns a spec a Drone named, in the Job's
   * worktree, and keeps what it captured as a set of its own beside the step's
   * frames. **It moves nothing on the Job.**
   *
   * **`spec` is one of `show_again.specs` on the Job's detail**, and without
   * one Fleet runs the last a Drone named. Fleet refuses any spec its own
   * record does not hold.
   *
   * **It answers when the press has landed**, which may be as long as the app
   * takes to start. Fleet runs it off the turn loop and bounds it by its own
   * Check budget, so Bridge does not add a wait of its own — see `NO_WAIT`.
   * Fleet refuses 409 before anything runs where it cannot, naming what is
   * missing; a press that ran and captured nothing answers with why.
   */
  showAgain: (jobId: string, spec?: string) => Promise<Outcome>;
  /**
   * Give one job a higher cost ceiling than the tier above it allows.
   *
   * **The act the `over_budget` label has always pointed at.** A job past its
   * cost cap waits at `queued` reading that label until somebody raises the cap
   * — the one reason a queued job carries that does not clear on its own — and
   * the only remedy before this was a machine-wide setting that governs every
   * job and takes on a restart.
   *
   * **It moves nothing.** No status, no step, no drone: admission was going to
   * start one and was refused for money, so the next turn starts it. What comes
   * back is the job, and the field worth reading is `queued_reason` —
   * `over_budget` before, absent or `waiting_on_resources` after.
   *
   * The figure is millionths of a dollar, which is the unit `JobSpend` reads
   * in. **It raises only**: a value at or under the cap in force is refused
   * before the request is sent, matching the 422 Fleet would answer, because a
   * press that reports success and leaves the job stopped is the failure this
   * exists against. Fleet refuses 409 on a terminal job, which has nothing left
   * to spend.
   */
  raiseCostCap: (jobId: string, costCapMicros: number) => Promise<Outcome>;
  /**
   * Let one Job take more turns than the tier above it allows.
   *
   * **The other ceiling `over_budget` folds, and until now the one with no
   * remedy.** A Job at its turn cap waits at `queued` reading the same label a
   * Job out of money reads, and raising the cost cap does not start it.
   *
   * **It moves nothing**, on `raiseCostCap`'s terms. What comes back is the
   * Job, and the field worth reading is `queued_reason`.
   *
   * The figure is a plain turn count, which is the unit `JobSpend` reads it in
   * — there is no conversion on this act. **It raises only**: a value at or
   * under the cap in force is refused before the request is sent, matching the
   * 422 Fleet would answer. Fleet refuses 409 on a terminal Job, which has no
   * turns left to take.
   */
  raiseTurnCap: (jobId: string, turnCap: number) => Promise<Outcome>;
  /**
   * Add a task to this Job's plan, from the Plan region's own eyebrow act.
   * `#897`. **Answers with the plan the add leaves**, not a plain `Outcome`
   * — `packages/screens/src/plan-edits.ts` says why — so the Plan region
   * redraws from the answer at once.
   */
  addTask: (jobId: string, add: AddTask) => Promise<PlanEditAnswer>;
  /**
   * Drop a task from this Job's plan, with a reason, from the row it is on.
   * `#897`. `addTask`'s own answer shape.
   */
  dropTask: (jobId: string, drop: DropTask) => Promise<PlanEditAnswer>;
  /**
   * Read one Job whole and keep it current, or `null` to stop.
   *
   * The renderer says which Job is open; main does the reading and republishes
   * it whenever an event names that Job. One call per open, not one per event.
   */
  watchJob: (jobId: string | null) => Promise<void>;
  /**
   * Watch one Job's turns, or `null` to stop.
   *
   * **Nothing is sent to the Drone and nothing can be.** This opens a socket
   * that only reads, closes it when the window closes it, and leaves the Job
   * exactly as it found it — a capability that could intervene would be Pilot,
   * which is a different act with a transition on the record.
   */
  observeJob: (jobId: string | null) => Promise<void>;
  /**
   * Read one running Check's log as it is written, or `null` to stop.
   *
   * **Read-only, like `observeJob`**, and its own entry because it is its own
   * socket. `kept` is the last component of a `CheckUnderway.output_path`, and
   * Fleet resolves it against the Checks it is running before opening anything.
   */
  followCheckOutput: (jobId: string | null, kept: string | null) => Promise<void>;
  /**
   * Read one merge line Check's log, running or ended, or `null` to stop.
   *
   * **Read-only, `followCheckOutput`'s terms**, by the line's three names and never a path:
   * Fleet finds the file from the branch's outcome and opens nothing else.
   */
  followLandCheck: (at: LandCheckAt | null) => Promise<void>;
  /**
   * Read one Job's transition history, or `null` to stop.
   *
   * **Its own entry because it is its own operation.** A history is not on
   * `JobDetail`, so folding it into `watchJob` would make every Job opened pay
   * for a surface that is folded away by default. Read-only, like the two above
   * it: a recorded move is a fact, and nothing here can add one.
   */
  readHistory: (jobId: string | null) => Promise<void>;
  /**
   * Read what the open Job holds on this machine, or `null` to stop.
   *
   * **Read-only and its own entry**, because it is its own operation on the
   * Rust side and for the same reason: `watchJob` is re-read on every event
   * naming the Job, and this walks a process table and a directory.
   */
  readResources: (jobId: string | null) => Promise<void>;
  /**
   * Keep the reading above live while Pulse is drawing it, or `null` to stop.
   *
   * **The board's poll, not the Job's.** `readResources` opens with the Job and
   * moves with its events; this is the 10 s tick that keeps the figures honest
   * while the Job is quiet, and it runs only for as long as somebody has the
   * board on screen. `resources-poll.ts`, `#1571`.
   */
  watchPulse: (jobId: string | null) => Promise<void>;
  /** Read the run sheet — Journey 9 — or `null` to stop. Opened by the sheet
   * and not by the Job, which is what `readDiff` no longer is. */
  watchRunSheet: (jobId: string | null) => Promise<void>;
  /** One run's output, or `null` to stop — opened for a run `startRun` just
   * began, or one the sheet is reopening onto in flight. */
  observeRun: (jobId: string | null, runId: string | null) => Promise<void>;
  /** Run one Check or Command in this Job's own worktree, narrowed or whole.
   * **A rehearsal**: no Evidence, nothing on the Job moves. Opens
   * `observeRun` for the caller the moment the run exists. */
  startRun: (jobId: string, body: StartRun) => Promise<Outcome>;
  /** End a run's process group. Its log keeps what printed. */
  stopRun: (jobId: string, runId: string) => Promise<Outcome>;
  /** Put back the files one run changed, from the snapshot taken just before
   * it. Refused while a Drone is working, while a run is in flight, on a run
   * already undone or with no snapshot. */
  undoRun: (jobId: string, runId: string) => Promise<Outcome>;
  /** Every earlier run from the sheet, newest first, and what would not read. */
  listRuns: (jobId: string) => Promise<RunListRead>;
  /** One run's log, read back as a window that says it is one. */
  getRunOutput: (jobId: string, runId: string) => Promise<RunOutputRead>;
  /**
   * Ask Fleet to go and look at this Job now, and say what it found.
   *
   * **The rung below intervene.** Every other act on a Job here changes it, so
   * a person who suspected one was wedged had one move. This one moves nothing
   * — what it leaves is a line in the Job's own log.
   *
   * It costs no model call. The answer arrives on `examination` rather than
   * coming back from the call, so a window reopened while a look was out still
   * gets it.
   */
  examineJob: (jobId: string) => Promise<void>;
  /**
   * Read what one Job's Drones claimed, or `null` to stop. Read-only, and its
   * own entry rather than folded into `readDiff`: they are two operations on
   * the Rust side because a surface wanting only the claims would otherwise
   * fetch a megabyte to read four lines.
   */
  readEvidence: (jobId: string | null) => Promise<void>;
  /**
   * Read one Job's worktree against its branch, or `null` to stop. Read-only.
   * **The one capability here that spends the patch bytes.**
   *
   * **Asked for more than once, and that is the point.** It said it was not
   * reachable by opening a Job, which stopped being true when the Produced
   * chapter started opening a file to what it actually wrote. The reading is
   * now taken when a Job opens, again on the press that opens the diff, and
   * again while that sheet is open and the file list moves under it — because
   * a patch read once on a running Job is the worktree as it was before its
   * Drone wrote. `packages/surfaces/jobs/src/produced.ts` holds what asks.
   */
  readDiff: (jobId: string | null) => Promise<void>;
  /**
   * Read one Check's own output, whole enough to read on the screen it is on.
   *
   * **It answers once rather than publishing**: a recorded output is finished,
   * and a person opening one Check is asking about that Check. `openArtifact`
   * hands the file to the operating system; this brings the lines in, so a
   * suite that went green for the wrong reason can be argued with in the app.
   *
   * `kept` is the row's own file name, off `output_path`. **Nothing here
   * composes a path** — `artifacts.ts` owns that rule — and Fleet resolves the
   * name against its own record. Read-only, like the reads above it.
   */
  readCheckOutput: (jobId: string, kept: string) => Promise<CheckOutputRead>;
  /** One kept brief a Judge or a gaming check was asked, by its file name. `readCheckOutput`'s shape. */
  readBrief: (jobId: string, name: string) => Promise<BriefRead>;
  /**
   * One Job's retro — `docs/concepts/retro.md`. `readBrief`'s shape: answered once to the
   * surface that asked, and asked again when the window regains focus, because nothing on
   * `/events` says a retro was written.
   */
  readRetro: (jobId: string) => Promise<RetroRead>;
  /**
   * Every written retro's items across Jobs, newest first, narrowed to this window's pick: the
   * open ones, or the saved Kit items under `accepted`.
   */
  readLessons: (state: "open" | "accepted") => Promise<LessonsRead>;
  /**
   * Agree with one retro item. An Armada or Manifest item proposes a Job at the approval gate;
   * a Kit item is saved under Accepted. Answers the item as it now stands.
   */
  agreeLesson: (lessonId: string) => Promise<LessonAnswer>;
  /** Disagree with one retro item: it is discarded. */
  disagreeLesson: (lessonId: string) => Promise<LessonAnswer>;
  readFrame: (jobId: string, kept: string) => Promise<FrameRead>;
  /**
   * `leftOut` and the Manifest reading for the repository New job's ask
   * answered — #959. `readCheckOutput`'s shape: a repository named by root rather
   * than a Job by id, answered once, and held nowhere — the Board stays on
   * All throughout, so nothing else on screen ever reads this repository.
   */
  readComposing: (repository: string) => Promise<ComposingRead>;
  /**
   * Where one frame's bytes stream from, as an address.
   *
   * **The only entry here that answers without asking anything.** Every read
   * above crosses a channel and comes back with bytes; a recording cannot be
   * held whole before it plays, so what crosses is a name on a scheme main
   * handles and forwards a span at a time.
   *
   * The renderer still reaches no port. `readFrame` beside it is what every
   * other kind of frame goes through, and neither one tells this window where
   * Fleet is.
   */
  frameStreamUrl: (jobId: string, kept: string) => string;
  /**
   * Take the work. **The counterpart to `approveDispatch`, at the other end of
   * the Job.** On the workflow's last step Fleet commits and delivers before
   * recording the Job done. Legal only at `awaiting_review`, like the two below.
   */
  approveReview: (jobId: string) => Promise<Outcome>;
  /**
   * Merge the pull request this Job's branch went out on, then take the work.
   *
   * **The only entry on this surface that writes into a repository Armada does
   * not own**, and the reason it exists under `auto_merge: never`: the policy
   * says no machine decides that work lands, and a person who merges on the
   * forge instead skips the checks Armada would have run against what landed.
   *
   * Its own entry rather than a flag on `approveReview`, for the reason the
   * three below are three: they differ in what happens to the world, and one
   * entry taking which would read as one act and perform four.
   */
  mergePullRequest: (jobId: string) => Promise<Outcome>;
  /**
   * Merge pressed while the forge's checks run: the forge is asked to merge when they pass, and
   * the Job stays at its gate until it has. The pull request as the forge shows it afterwards is
   * `pullRequest` on the outcome. Since protocol 23.74.
   */
  autoMergePullRequest: (jobId: string) => Promise<Outcome>;
  /** Start the pull request's failed CI runs again. A forge write, only from a press. #905. */
  rerunFailedChecks: (jobId: string) => Promise<Outcome>;
  /** Send the branch back for a Drone to find out why CI failed. #905. */
  investigateFailedChecks: (jobId: string) => Promise<Outcome>;
  /** Queue a Job after this one lands, from a For context finding. #906. */
  queueAfterFinding: (jobId: string, finding: string) => Promise<Outcome>;
  /** File the issue a person confirmed. Written as them, so only from their confirm. #906. */
  fileFindingIssue: (jobId: string, finding: string, title: string, body: string) => Promise<Outcome>;
  /** Open the issue a finding became. Main reads its address; the renderer sends none. #906. */
  openFindingIssue: (jobId: string, finding: string) => Promise<Followed>;
  /**
   * Send the work back with a note. **The Job comes back `running`**, same step,
   * same Drone — nothing is spawned and nothing done is thrown away.
   */
  requestChanges: (jobId: string, note: string, withWalkNotes?: boolean) => Promise<Outcome>;
  /** Take back a walk note not yet sent. Capturing one is the walk window's own. */
  removeWalkNote: (jobId: string, noteId: string) => Promise<Outcome>;
  /**
   * A verdict on the work, and the Job is over. **Terminal, and it ends the
   * Drone** — that is what separates it from `requestChanges`, and it is not
   * `killJob`, which clears the Board and carries no verdict at all. Three
   * entries and not one taking which: that would read as one act and perform
   * three, and the three differ by whether anything survives.
   */
  rejectWork: (jobId: string) => Promise<Outcome>;
  /**
   * What people wrote on one Job's open pull request, or `null` to stop.
   *
   * **Read-only, and the one read on this surface that reaches a forge.** It is
   * its own entry beside `readEvidence` and `readDiff` for their reason: three
   * operations, opened by the surfaces that draw them, and this one costs a
   * process and a network rather than a record or a worktree.
   */
  readRemarks: (jobId: string | null) => Promise<void>;
  /**
   * Hand the comments a person picked off the pull request to a Drone.
   * Nothing is written back onto the pull request.
   *
   * **A fifth entry rather than a flag on `requestChanges`.** What reaches
   * Fleet is a set of handles off a forge, not a person's words — the words are
   * read from the forge on the press, so nothing here decides what a Drone is
   * told. What it does to the Job is what `requestChanges` does.
   */
  takeUpRemarks: (jobId: string, remarks: string[]) => Promise<Outcome>;
  /** Dismiss a finding the review raised, with the reason. It moves nothing. #907. */
  dismissFinding: (jobId: string, finding: string, reason: string) => Promise<Outcome>;
  /**
   * Open one of a Job's artifacts in whatever the OS opens it with.
   *
   * **One entry taking which, where the kills and the decisions are three.**
   * Those are split because they differ in what survives them; these three
   * differ only in which file, they change nothing about the Job, and a Job id
   * plus a word from a closed set is deliberately less than a path — main
   * derives the path, so the renderer never holds the argument that matters.
   *
   * The branch is absent from `Artifact` and stays a copy: it is served rather
   * than derived, and it is not a path.
   */
  openArtifact: (jobId: string, what: Artifact) => Promise<Opened>;
  /**
   * Open the pull request Fleet opened for this Job, in whatever browses the
   * web on this machine.
   *
   * **A Job id and nothing else, which is `openArtifact`'s rule.** Main reads
   * the address off the reading it published and checks it is a web address
   * before handing it over, so the renderer never holds the argument that
   * decides what opens — an address arriving from a click handler is the one
   * string that would turn this into arbitrary URL handling.
   *
   * **Its own entry rather than a fourth `Artifact`.** That set is files, main
   * derives a path for each, and every one of them lands on this machine; this
   * one is served rather than derived and it leaves the app. One capability
   * covering both would read as one act and perform two.
   */
  openPullRequest: (jobId: string) => Promise<Followed>;
  /**
   * Open one comment on the pull request, in whatever browses the web on this
   * machine.
   *
   * **A Job id and a comment id, never an address** — `openPullRequest`'s rule
   * exactly. Main looks the comment up in the remarks reading it published,
   * reads its own `url` off that and checks it is a web address before handing
   * it over, so the renderer never holds the string that decides what opens.
   *
   * `undefined` where the comment carries no address: the caller draws no link
   * for one, rather than a link that opens nothing.
   */
  openRemarkLink: (jobId: string, remarkId: string) => Promise<Followed>;
};

export type JobsState = {
  /** Jobs with an approval in flight. What stops a second dispatch. */
  approving: string[];
  /**
   * The proposal this window is waiting on, or `null` where it is waiting on
   * none.
   *
   * **The one piece of state here that is not about a Job**, because a proposal
   * is the interval before any Job exists. It appears when Fleet says the call
   * went out, moves as the call gets somewhere, and is `null` again the moment
   * the call comes back — however it came back.
   *
   * **This window's own, matched on the token it sent.** Fleet publishes every
   * proposal on one stream and two windows may be dispatching at once; a state
   * that folded whichever arrived last would draw somebody else's call as
   * yours, and offer a stop that killed it.
   */
  proposing: ProposalInFlight | null;
  /**
   * The one Job read whole, where a detail is open.
   *
   * **Published, not fetched by the component that draws it.** The detail is
   * re-read whenever an event names its Job, which is what makes the rail move
   * without a reload — a renderer holding its own copy would go stale the
   * moment a step advanced.
   */
  watched: Watched;
  /**
   * The Job being watched turn by turn, where somebody opened one.
   *
   * **Its own socket, and its own piece of state.** Transcript rows arrive at
   * Drone speed, and putting them on the stream the Board is drawn from would
   * evict the state changes that draw it. Separate here for the same reason.
   */
  observed: Observed;
  /**
   * What Fleet has done to the Job being watched — its own log, live.
   *
   * **Its own piece of state beside `observed`, and its own socket.** The two
   * answer different questions and neither substitutes for the other: a Drone's
   * transcript exists only while a Drone does, and this is the only thing there
   * is to draw while a worktree is being cut. A Job with all its steps
   * `not_started` has an empty `observed` and a full `journalled`, which is
   * exactly the moment somebody opens the panel.
   */
  journalled: Journalled;
  /**
   * The running Check's log somebody opened, as it is written. **Its own
   * socket**, for `journalled`'s reason: a Check's output is a file still
   * growing, and the event stream is bounded to keep payloads that size off
   * it. `following.ts`.
   */
  followed: FollowedLog;
  /**
   * What the open Job's Drone has changed in its worktree.
   *
   * **Only the open Job's, and only while the event arrives.**
   * `job.files_changed` is published for every Job on the one stream; keeping
   * every Job's footprint would make the Board pay for a detail nobody has
   * open, which is the thing this read is meant to stay off.
   */
  footprint: Footprint;
  /**
   * The moment the open Job's Drone handed in, before the gate started.
   *
   * **Only the open Job's, `footprint`'s terms** — `evidence.submitted` is
   * published for every Job on the one stream, and a submission on a Job
   * nobody has open moves nothing on the Board. What was submitted is
   * `evidence`, which is fetched; this is only that it happened. `#813`.
   */
  handed: Handed;
  /**
   * One Job's transition history, where a surface asked for one.
   *
   * **Read when it is asked for, not on every open.** It is its own operation
   * for that reason: a detail is fetched to draw a summary and a history has no
   * bound — it grows for as long as the Job lives, and a retried step is a row
   * per attempt plus the moves around it.
   */
  history: History;
  /**
   * What one Job's Drones claimed, where a surface asked for it. The cheap half
   * of the pair, and still asked for rather than paid for on every open.
   */
  evidence: Evidence;
  /**
   * One Job's worktree against the branch it was cut from, where a surface asked
   * for it. **The expensive half, and the one place the patch bytes are spent.**
   * `crates/adapter-traits/src/work_product.rs` splits it off the file list
   * because the bytes are large and most steps ask no semantic question; this is
   * read on the act they were split for, never folded into `watched`, which is
   * re-read every time an event names the open Job.
   */
  diff: Diff;
  /**
   * What people wrote on one Job's open pull request, where the surface a
   * person decides on asked for it. **The one read in this state that costs a
   * forge** — nothing takes it on a timer and no event refreshes it, so it is
   * opened by that surface and dropped when it closes.
   */
  remarks: Remarks;
  /**
   * What the open Job holds on this machine — its processes, what each is
   * burning, and the disk its worktree has taken.
   *
   * **Opened with the Job and re-read while it is open**, which is what makes
   * it a live panel rather than a snapshot: a figure that stopped moving while
   * a Job ran would be a panel claiming a stall that is not there. It keeps its
   * last good reading through a failed re-read for `watched`'s reason — a
   * blanked panel reads as a Job holding nothing, which is the exact answer
   * this exists to make loud.
   */
  resources: Holds;
  /**
   * Every Drone the open Job has had, running or not — `list_job_drones`.
   *
   * **Opened with the Job and re-read on every event naming it**, for
   * `resources`' reason, and kept through a failed re-read for it too.
   */
  jobDrones: Crewed;
  /**
   * What Fleet found when somebody pressed for a look — and only then.
   *
   * **Not read on opening a Job.** It is a thing a person did, it costs a
   * process table and a directory walk, and an answer that appeared without
   * anybody asking would be the automatic bound rather than the person's half
   * of it. It stays on screen until they press again or leave the Job.
   */
  examination: Examination;
  /** The run sheet — Journey 9. Opened with the sheet, not the Job, `diff`'s
   * rule: most Jobs are never rehearsed, so nothing pays for one nobody asked. */
  runSheet: RunSheetRead;
  /** The run a window is reading, as it prints — its own socket, `followed`'s
   * shape one subject over. One at a time. */
  runFollowed: RunFollowed;
  /**
   * The line `armada land` keeps in each served repository that has one, or `null` before Fleet
   * has answered. **Read once per connection and replaced whole by `merge_lines.changed`**,
   * `servers`' terms: Fleet reads the files, and Bridge keeps no timer of its own. Shared, not
   * this window's own: the panels fold it against the window's pick (`mergeLineViews`).
   */
  mergeLines: MergeLines | null;
  /**
   * The merge line Check's log somebody opened, running or ended. **Its own socket**, `followed`'s
   * reason, keyed by the line's names rather than a Job. `land-following.ts`.
   */
  landFollowed: FollowedLandLog;
};

export const JOBS_NOTHING_YET: JobsState = {
  approving: [],
  proposing: null,
  watched: { state: "none" },
  observed: { state: "none" },
  journalled: { state: "none" },
  followed: { state: "none" },
  footprint: { state: "none" },
  handed: { state: "none" },
  history: { state: "none" },
  evidence: { state: "none" },
  diff: { state: "none" },
  remarks: { state: "none" },
  resources: { state: "none" },
  jobDrones: { state: "none" },
  examination: { state: "none" },
  runSheet: { state: "none" },
  runFollowed: { state: "none" },
  mergeLines: null,
  landFollowed: { state: "none" },
};

export const JOBS_CHANNELS = {
  proposeFromRequest: "bridge:propose-from-request",
  stopProposal: "bridge:stop-proposal",
  stageAttachment: "bridge:stage-attachment",
  searchFiles: "bridge:search-files",
  approveDispatch: "bridge:approve-dispatch",
  listBranches: "bridge:list-branches",
  redispatchJob: "bridge:redispatch-job",
  killDrone: "bridge:kill-drone",
  killJob: "bridge:kill-job",
  parkJob: "bridge:park-job",
  resumeJob: "bridge:resume-job",
  fixMain: "bridge:fix-main",
  killProcess: "bridge:kill-process",
  killProcesses: "bridge:kill-processes",
  pilotTask: "bridge:pilot-task",
  restartTask: "bridge:restart-task",
  editTask: "bridge:edit-task",
  movePlan: "bridge:move-plan",
  approveWave: "bridge:approve-wave",
  editJob: "bridge:edit-job",
  setLandingTarget: "bridge:set-landing-target",
  toProposer: "bridge:to-proposer",
  redirectDrone: "bridge:redirect-drone",
  answerQuestion: "bridge:answer-question",
  answerCommand: "bridge:answer-command",
  // What a command does, read for the person deciding about it. A read: it
  // moves nothing, and the three answers are live while it is out.
  explainCommand: "bridge:explain-command",
  setWhenBlocked: "bridge:set-when-blocked",
  answerJudge: "bridge:answer-judge",
  setWhenRefused: "bridge:set-when-refused",
  setModel: "bridge:set-model",
  setReviewModel: "bridge:set-review-model",
  removeAllowedCommand: "bridge:remove-allowed-command",
  restartStep: "bridge:restart-step",
  overrideVerdict: "bridge:override-verdict",
  rerunGate: "bridge:rerun-gate",
  rerunChecks: "bridge:rerun-checks",
  showAgain: "bridge:show-again",
  raiseCostCap: "bridge:raise-cost-cap",
  raiseTurnCap: "bridge:raise-turn-cap",
  addTask: "bridge:add-task",
  dropTask: "bridge:drop-task",
  watchJob: "bridge:watch-job",
  observeJob: "bridge:observe-job",
  followCheckOutput: "bridge:follow-check-output",
  followLandCheck: "bridge:follow-land-check",
  readHistory: "bridge:read-history",
  readEvidence: "bridge:read-evidence",
  readResources: "bridge:read-resources",
  watchPulse: "bridge:watch-pulse",
  watchRunSheet: "bridge:watch-run-sheet",
  observeRun: "bridge:observe-run",
  startRun: "bridge:start-run",
  stopRun: "bridge:stop-run",
  undoRun: "bridge:undo-run",
  listRuns: "bridge:list-runs",
  getRunOutput: "bridge:get-run-output",
  examineJob: "bridge:examine-job",
  readDiff: "bridge:read-diff",
  readRemarks: "bridge:read-remarks",
  readCheckOutput: "bridge:read-check-output",
  readBrief: "bridge:read-brief",
  readRetro: "bridge:read-retro",
  readLessons: "bridge:read-lessons",
  agreeLesson: "bridge:agree-lesson",
  disagreeLesson: "bridge:disagree-lesson",
  readFrame: "bridge:read-frame",
  approveReview: "bridge:approve-review",
  mergePullRequest: "bridge:merge-pull-request",
  autoMergePullRequest: "bridge:auto-merge-pull-request",
  rerunFailedChecks: "bridge:rerun-failed-checks",
  investigateFailedChecks: "bridge:investigate-failed-checks",
  queueAfterFinding: "bridge:queue-after-finding",
  fileFindingIssue: "bridge:file-finding-issue",
  openFindingIssue: "bridge:open-finding-issue",
  requestChanges: "bridge:request-changes",
  removeWalkNote: "bridge:remove-walk-note",
  rejectWork: "bridge:reject-work",
  takeUpRemarks: "bridge:take-up-remarks",
  dismissFinding: "bridge:dismiss-finding",
  openArtifact: "bridge:open-artifact",
  openPullRequest: "bridge:open-pull-request",
  openRemarkLink: "bridge:open-remark-link",
  // New job's own reads for the repository its ask answered, on All — #959.
  // A request/response like `readCheckOutput`, answered to the
  // caller and published nowhere: `BridgeState` carries nothing about it.
  readComposing: "bridge:read-composing",
} as const;
