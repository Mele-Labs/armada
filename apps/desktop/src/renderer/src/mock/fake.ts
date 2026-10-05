// A `window.armada` with no main process behind it, answering from a scenario.
//
// **Typed as `BridgeApi` member by member, never a `Proxy`**, so a capability
// added to the preload fails typecheck here until this answers it. What each
// kind of call answers, and why no more: `docs/practices/running-locally.md`,
// *Bridge on a mock Fleet*.

import { PROTOCOL_VERSION, refusedWith } from "@armada/protocol";
import type { JobSummary, LessonAnswer, Outcome, RetroItem, WorkPlan } from "@armada/protocol";
import type { ArcDraft } from "@armada/screens/src/fixtures/build/arc";
import type { GroupView } from "@armada/screens/src/draft/group";
import type { PlanEditAnswer } from "@armada/screens/src/plan-edits";

import type { BridgeApi } from "../../../shared/api";
import type { BridgeState, Summons } from "../../../shared/bridge";
import { unanswered } from "./scenario";
import type { Scenario } from "./scenario";
import { keeping } from "./studio-fleet";
import { reshaped, rescued, scoutRead } from "./slots-fleet";
import type { RescueOutcome } from "@armada/screens/src/slot-rescue";
import { answered, listed } from "./lessons-fleet";
import { approvedAs, edited, landingRefusal, landingTargetSet, sentBack, tuningRefusal, waveJobEdited } from "./approval-fleet";
import {
  groupsAdding,
  groupsDropping,
  groupsEditing,
  groupsMoving,
  groupsRestarting,
  nextTaskId,
  planAdding,
  planDropping,
  planEditing,
  planMoving,
  planRestarting,
} from "./plan-fleet";

const OK: Outcome = { ok: true };
/**
 * How far apart a fixture's `arriving` rows land. Long enough that a walk has
 * opened a transcript's panel before the first one, short enough that the
 * walk's five-second wait for it holds.
 */
const ARRIVING_MS = 4000;

/** A fake's draft as it stands, and a way to hear it change. */
export type LiveDraft = {
  current: () => ArcDraft | undefined;
  subscribe: (onDraft: () => void) => () => void;
};

/**
 * Each fake's own draft, by the api it answers as. **Beside `BridgeApi` rather
 * than on it**: the draft is the mock's stand-in for reads Fleet does not serve
 * (`moment.ts`' `draft`), and the preload has no such member to type it by.
 */
const DRAFTS = new WeakMap<BridgeApi, LiveDraft>();

/** The draft `api`'s fake holds, where `api` is one. `mount.tsx` draws from it. */
export function liveDraft(api: BridgeApi): LiveDraft | undefined {
  return DRAFTS.get(api);
}

/** A window's own `window.armada`, over one scenario. Each call makes a fresh one. */
export function fakeBridge(scenario: Scenario): BridgeApi {
  let state: BridgeState = scenario.state;
  const listeners = new Set<(state: BridgeState) => void>();
  const summoners = new Set<(to: Summons) => void>();
  let summoned = false;
  let draft = scenario.draft;
  /** The pool as acts on it left it, read again after each as main does. */
  let held = scenario.held;
  const drafters = new Set<() => void>();

  /** Publish a change — on a microtask, because main's reach a window over IPC, never inside the call. */
  function publish(change: Partial<BridgeState>): void {
    state = { ...state, ...change };
    const now = state;
    queueMicrotask(() => listeners.forEach((listener) => listener(now)));
  }

  /** One field on one Job, and on its detail where that Job is the one open. */
  function move(jobId: string, change: Partial<JobSummary>): void {
    const watched = state.watched;
    publish({
      jobs: state.jobs.map((job) => (job.id === jobId ? { ...job, ...change } : job)),
      watched:
        watched.state === "read" && watched.jobId === jobId
          ? { ...watched, detail: { ...watched.detail, job: { ...watched.detail.job, ...change } } }
          : watched,
    });
  }

  /**
   * A person's own add or drop, folded where main's `foldPlan` folds the plan
   * Fleet answers with, and into the draft groups an arc board draws instead.
   * **Refused where no plan is open to change**, as Fleet refuses a Job with
   * no plan recorded.
   */
  function editPlan(
    jobId: string,
    route: string,
    plan: (was: WorkPlan) => WorkPlan | undefined,
    groups: (was: GroupView[]) => GroupView[],
  ): PlanEditAnswer {
    const watched = state.watched;
    const was = watched.state === "read" && watched.jobId === jobId ? watched.detail.work_plan : undefined;
    const now = was === undefined ? undefined : plan(was);
    if (watched.state !== "read" || now === undefined) return refused(path(jobId, route));
    publish({ watched: { ...watched, detail: { ...watched.detail, work_plan: now } } });
    if (jobId === scenario.opens && draft?.groups !== undefined) {
      draft = { ...draft, groups: groups(draft.groups) };
      queueMicrotask(() => drafters.forEach((onDraft) => onDraft()));
    }
    return { ok: true, plan: now };
  }

  /** The retro items as answers left them. */
  let lessons = scenario.lessons ?? [];

  /** One answer to one item. An agreed Armada or Manifest item puts its Job on the Board at the gate. */
  function answerLesson(id: string, answer: "agree" | "disagree"): LessonAnswer {
    const taken = answered(lessons, id, answer, state.jobs, new Date().toISOString());
    lessons = taken.lessons;
    if (taken.job !== undefined) publish({ jobs: [...state.jobs, taken.job] });
    return taken.answer;
  }

  function forget(jobIds: readonly string[]): void {
    publish({ jobs: state.jobs.filter((job) => !jobIds.includes(job.id)) });
  }

  /** This window's Fleet for Studios, over the Studios the scenario names. */
  const studios = keeping(scenario.studios).routes({ state: () => state, publish });

  const readsOf = (jobId: string) => scenario.reads[jobId];
  const path = (jobId: string, route = "") => `/jobs/${encodeURIComponent(jobId)}${route}`;
  const failed = (jobId: string, route: string) =>
    ({ state: "failed", jobId, outcome: unanswered(path(jobId, route)) }) as const;
  const refused = (route: string) => ({ ok: false, outcome: unanswered(route) }) as const;
  const unread = (route: string) => ({ state: "failed", outcome: unanswered(route) }) as const;
  const nothing = { state: "none" } as const;

  const api: BridgeApi = {
    protocolVersion: () => PROTOCOL_VERSION,
    state: async () => state,
    subscribe: (onState) => {
      listeners.add(onState);
      return () => listeners.delete(onState);
    },
    // The stand-in walk window sets the dim itself — `walk-window.tsx`.
    onWalkFocus: () => () => {},

    // **The row appears and the call never answers.** A dispatched request is a
    // Job from the press — `job-statuses.toml`, `proposing` — and that Job is
    // the only thing the mock can honestly mint: no workflow, no steps and no
    // plan, so there is nothing here to invent. The promise is left out because
    // the proposer never answers in the mock, which is what `in_code = "Not
    // yet"` means; nothing is waiting on it, since the press left the composer.
    proposeFromRequest: (request) => {
      const at = new Date().toISOString();
      const row = proposingRow(request, at, state.jobs);
      publish({ jobs: [...state.jobs, row] });
      return new Promise(() => {});
    },
    stopProposal: async () => OK,
    stageAttachment: async (_bytes, filename) => ({ path: filename }),
    searchFiles: async () => [],

    // The body applied as Fleet applies it (23.8), on the Job open; a row not
    // open only moves to `queued`, which is all the Board draws of it.
    approveDispatch: async (jobId, approval) => {
      const watched = state.watched;
      if (watched.state !== "read" || watched.jobId !== jobId) return (move(jobId, { status: "queued" }), OK);
      const refusal = tuningRefusal(watched.detail, approval) ?? landingRefusal(approval?.landing);
      if (refusal !== undefined) return refusal;
      const detail = approvedAs(watched.detail, approval, new Date().toISOString());
      publish({
        jobs: state.jobs.map((job) => (job.id === jobId ? { ...job, ...detail.job } : job)),
        watched: { ...watched, detail },
      });
      return OK;
    },
    // The proposal back to the proposer with a note (`to_proposer`, 23.25).
    toProposer: async (jobId, body) => {
      const watched = state.watched;
      if (watched.state !== "read" || watched.jobId !== jobId) return OK;
      const back = sentBack(watched.detail, body);
      if ("ok" in back) return back;
      publish({ watched: { ...watched, detail: back } });
      return OK;
    },
    // Where an approved Job lands, once (`set_landing_target`, 23.22).
    setLandingTarget: async (jobId, target) => {
      const watched = state.watched;
      if (watched.state !== "read" || watched.jobId !== jobId) return OK;
      const set = landingTargetSet(watched.detail, target);
      if ("ok" in set) return set;
      publish({ watched: { ...watched, detail: set } });
      return OK;
    },
    // The repository's branches, where a Job of it carries them (#1605).
    listBranches: async (manifestId) => {
      const held = Object.values(scenario.reads).find(
        (one) => one.job.owner_manifest_id === manifestId && one.branches !== undefined,
      );
      const asked = `/manifest/branches?manifest_id=${encodeURIComponent(manifestId)}`;
      return held?.branches === undefined ? refused(asked) : { ok: true, branches: held.branches };
    },
    redispatchJob: async () => OK,
    killDrone: async () => OK,
    killJob: async (jobId) => (move(jobId, { status: "killed" }), OK),
    // Accepted, as Fleet answers a pid in the Job's tree (#1647). The mock
    // takes no second reading, so the row stays where Fleet's would drop.
    killProcess: async () => OK,
    killProcesses: async () => OK,
    // A failed task's Pilot, #250 — not served by Fleet yet, so
    // answered as Fleet's router answers a route it has none for: a bare 404,
    // through the parser main's `ask` uses.
    pilotTask: async (jobId, taskId) =>
      refusedWith(404, "", { method: "POST", path: path(jobId, `/tasks/${taskId}/pilot`) }),
    // Served since 23.4 (#1656): the failed task is worked again.
    restartTask: async (jobId, taskId) => {
      const failedInDraft =
        draft?.groups?.some((group) => group.tasks.some((task) => task.id === taskId && task.state === "failed")) ??
        false;
      const answer = editPlan(
        jobId,
        `/tasks/${taskId}/restart`,
        (was) =>
          planRestarting(was, taskId, failedInDraft, state.watched.state === "read" ? state.watched.detail.steps : []),
        (was) => groupsRestarting(was, taskId),
      );
      return answer.ok ? OK : answer.outcome;
    },
    // Served since 23.6 (#1657): the fields sent, over the task's own.
    editTask: async (jobId, taskId, edit) => {
      const editableInDraft =
        draft?.groups?.some((group) =>
          group.tasks.some((task) => task.id === taskId && (task.state === "open" || task.state === "failed")),
        ) ?? false;
      const answer = editPlan(
        jobId,
        `/tasks/${taskId}/edit`,
        (was) => planEditing(was, taskId, edit, editableInDraft),
        (was) => groupsEditing(was, taskId, edit),
      );
      return answer.ok ? OK : answer.outcome;
    },
    // A drop on the plan, served since 23.4 (#1685): placed by `after`.
    movePlan: async (jobId, move) => {
      const answer = editPlan(
        jobId,
        "/plan/move",
        (was) => planMoving(was, move),
        (was) => groupsMoving(was, move),
      );
      return answer.ok ? OK : answer.outcome;
    },
    // An Epic's plan approved with its wave, served since 23.11 (#1694): every
    // Job named leaves its gate, and the Epic is queued behind them.
    approveWave: async (jobId, wave) => {
      wave.jobs.forEach((one) => move(one, { status: "queued" }));
      move(jobId, { status: "queued" });
      const drawn = draft?.wave;
      if (drawn !== undefined) {
        const released = (one: (typeof drawn.jobs)[number]) =>
          wave.jobs.includes(one.job) ? { ...one, status: "queued" } : one;
        draft = { ...draft, wave: { ...drawn, jobs: drawn.jobs.map(released) } };
        queueMicrotask(() => drafters.forEach((onDraft) => onDraft()));
      }
      return OK;
    },
    // A proposal's words saved without releasing it (`edit_job`, 23.8): on the
    // row, on the Job where it is open, and on the wave's panel that sent it.
    editJob: async (jobId, edit) => {
      if (edit.title !== undefined) move(jobId, { title: edit.title });
      const watched = state.watched;
      if (watched.state === "read" && watched.jobId === jobId) {
        publish({ watched: { ...watched, detail: edited(watched.detail, edit) } });
      }
      const wave = draft?.wave;
      if (wave !== undefined) {
        draft = { ...draft, wave: { ...wave, jobs: wave.jobs.map((one) => (one.job === jobId ? waveJobEdited(one, edit) : one)) } };
        queueMicrotask(() => drafters.forEach((onDraft) => onDraft()));
      }
      return OK;
    },
    clearTerminalJobs: async (jobIds) => {
      const at = new Date().toISOString();
      jobIds.forEach((jobId) => move(jobId, { reclaimed_at: at }));
      return { reclaimed: [], failed: [] };
    },
    forgetTerminalJobs: async (jobIds) => (forget(jobIds), { cleared: [...jobIds], failed: [] }),
    reclaimWorktree: async (jobId) => (move(jobId, { reclaimed_at: new Date().toISOString() }), OK),
    changeSlotPool: async (manifestId, change) => {
      if (held === undefined) return unanswered(`/worktrees/slots?manifest_id=${manifestId}`);
      const after = reshaped(held, manifestId, change);
      held = after.held;
      if (state.held.state === "read") publish({ held: { state: "read", held } });
      return after.outcome;
    },
    rescueSlot: async (manifestId, rescue) => {
      if (held === undefined) return unanswered(`/worktrees/slots/rescue?manifest_id=${manifestId}`) as RescueOutcome;
      const after = rescued(held, manifestId, rescue);
      held = after.held;
      // A Pick up proposes a Job from the press, as a dispatched request does.
      const proposing =
        after.proposed === undefined ? {} : { jobs: [...state.jobs, proposingRow(after.proposed.split("\n")[0] ?? "", new Date().toISOString(), state.jobs)] };
      publish(state.held.state === "read" ? { ...proposing, held: { state: "read", held } } : proposing);
      return after.outcome;
    },
    deleteBranch: async () => OK,
    forgetJob: async (jobId) => (forget([jobId]), OK),
    redirectDrone: async () => OK,
    answerQuestion: async () => OK,
    answerCommand: async () => OK,
    answerHelmCall: async () => OK,
    explainCommand: async (jobId, callId) => refused(path(jobId, `/calls/${callId}/explain`)),
    setWhenBlocked: async () => OK,
    answerJudge: async () => OK,
    setWhenRefused: async () => OK,
    setModel: async (jobId, model) => {
      if (model !== null) move(jobId, { model });
      return OK;
    },
    setReviewModel: async () => OK,
    removeAllowedCommand: async () => OK,
    restartStep: async () => OK,
    overrideVerdict: async () => OK,
    rerunGate: async () => OK,
    rerunChecks: async () => OK,
    showAgain: async () => OK,
    raiseCostCap: async () => OK,
    raiseTurnCap: async () => OK,
    saveLimits: async () => OK,
    savePreference: async () => OK,
    fileReport: async () => OK,
    addTask: async (jobId, add) => {
      let id = "";
      return editPlan(
        jobId,
        "/add_task",
        (was) => ((id = nextTaskId(was, draft?.groups)), planAdding(was, id, add)),
        (was) => groupsAdding(was, id, add),
      );
    },
    dropTask: async (jobId, drop) =>
      editPlan(
        jobId,
        "/drop_task",
        (was) => (was.tasks.some((task) => task.id === drop.task) ? planDropping(was, drop) : undefined),
        (was) => groupsDropping(was, drop),
      ),

    // The footprint and the hand-in are pushed about the open Job, so they
    // arrive with it, and main reads its Drones with it.
    watchJob: async (jobId) => {
      const reads = jobId === null ? undefined : readsOf(jobId);
      publish({
        watched: jobId === null ? nothing : (reads?.watched ?? failed(jobId, "")),
        jobDrones: reads?.jobDrones ?? nothing,
        footprint: reads?.recorded.footprint ?? nothing,
        handed: reads?.recorded.handed ?? nothing,
      });
    },
    observeJob: async (jobId) => {
      publish({ observed: jobId === null ? nothing : (readsOf(jobId)?.observed ?? nothing) });
      // What a writing Drone sends next, one row at a time, onto the socket
      // that is still this Job's. A row already held is not sent twice, so a
      // socket opened again does not double the tail.
      (jobId === null ? [] : (readsOf(jobId)?.arriving ?? [])).forEach((row, at) => {
        setTimeout(() => {
          const now = state.observed;
          if (!("turns" in now) || now.jobId !== jobId || now.turns.rows.some((one) => one.seq === row.seq)) return;
          publish({ observed: { ...now, turns: { ...now.turns, rows: [...now.turns.rows, row] } } });
        }, ARRIVING_MS * (at + 1));
      });
    },
    followCheckOutput: async () => publish({ followed: nothing }),
    followLandCheck: async () => publish({ landFollowed: nothing }),
    // A fixture with no history is one whose story never asked for it, and `none` is what it draws.
    readHistory: async (jobId) =>
      publish({ history: jobId === null ? nothing : (readsOf(jobId)?.history ?? nothing) }),
    readResources: async (jobId) =>
      publish({
        resources: jobId === null ? nothing : (readsOf(jobId)?.resources ?? failed(jobId, "/resources")),
      }),
    // The mock publishes once and never moves, so the poll has nothing to take
    // again — what it has to be here for is that the board can ask.
    watchPulse: async () => {},
    watchRunSheet: async (jobId) =>
      publish({ runSheet: jobId === null ? nothing : failed(jobId, "/runs/sheet") }),
    observeRun: async () => publish({ runFollowed: nothing }),
    startRun: async () => OK,
    stopRun: async () => OK,
    undoRun: async () => OK,
    listRuns: async (jobId) => refused(path(jobId, "/runs")),
    getRunOutput: async (jobId, runId) => refused(path(jobId, `/runs/${runId}/output`)),

    watchCheckoutRunSheet: async (want) =>
      publish({ checkoutRunSheet: want ? unread("/manifest/runs/sheet") : nothing }),
    observeCheckoutRun: async () => publish({ checkoutRunFollowed: nothing }),
    startCheckoutRun: async () => OK,
    stopCheckoutRun: async () => OK,
    undoCheckoutRun: async () => OK,
    listCheckoutRuns: async () => refused("/manifest/runs"),
    getCheckoutRunOutput: async (runId) => refused(`/manifest/runs/${runId}/output`),
    getCheckoutRunDiff: async (runId) => refused(`/manifest/runs/${runId}/diff`),
    watchManifestDrift: async (want) => publish({ manifestDrift: want ? unread("/manifest/drift") : nothing }),
    // Held for the life of the window: a failure here would draw every surface's Fleet panel in trouble.
    watchOverview: async () => undefined,
    startCheckoutVerify: async () => OK,

    readManifestFile: async () => refused("/manifest/file"),
    saveManifestFile: async () => unread("/manifest/file"),
    editManifest: async () => unread("/manifest/edit"),
    readManifestSpend: async () => refused("/manifest/spend"),

    readRepositoryScan: async () => refused("/repositories/scan"),
    readManifestProposals: async () => refused("/manifest/proposals"),
    editManifestProposal: async () => unread("/manifest/proposals"),
    writeManifestProposal: async () => unread("/manifest/proposals"),

    listRepositoryAllowedCommands: async () => refused("/manifest/allowed-commands"),
    removeRepositoryAllowedCommand: async () => refused("/manifest/allowed-commands"),

    readKitInventory: async () => refused("/kit/inventory"),
    listKitServers: async () => refused("/kit/servers"),
    addKitServer: async () => refused("/kit/servers"),
    forgetKitServer: async () => refused("/kit/servers"),
    setKitServerReach: async () => refused("/kit/servers"),
    setManifestServerReach: async () => refused("/kit/servers"),

    // The rail's pick is this window's own, so it moves here as it does in main.
    pickRepository: async (root) => publish({ repository: root }),

    chooseFolder: async () => null,
    resolveFolder: async () => null,
    addRepository: async () => unread("/repositories"),
    cloneRepository: async () => unread("/repositories/clone"),

    startServer: async () => OK,
    stopServer: async () => OK,
    openServerLink: async () => ({ ok: false, why: "no_address" }),
    openLink: async () => ({ ok: true }),
    // The capture window is a second window main opens — #1294. The mock has
    // none, so this says what a Studio nothing is holding would say.
    openCaptureWindow: async () => ({ ok: false, why: "no_studio" }),
    captureWindow: {
      read: () => () => {},
      arm: async () => null,
      aim: async () => null,
      hold: async () => null,
      release: async () => {},
      save: async () => OK,
      reload: async () => {},
      followRefused: async () => {},
      scroll: () => {},
    },
    // A Studio starting one entry — #1289, #1345. The mock Fleet answers, so
    // what these do here is what every other unstubbed act does: nothing.
    startStudioRun: async () => ({ ok: true }) as Outcome,
    startStudioServer: async () => ({ ok: true }) as Outcome,
    examineJob: async (jobId) => publish({ examination: failed(jobId, "/examine") }),
    readEvidence: async (jobId) =>
      publish({
        evidence: jobId === null ? nothing : (readsOf(jobId)?.recorded.evidence ?? failed(jobId, "/evidence")),
      }),
    readDiff: async (jobId) =>
      publish({ diff: jobId === null ? nothing : (readsOf(jobId)?.recorded.diff ?? failed(jobId, "/diff")) }),
    readCheckOutput: async (jobId, kept) =>
      readsOf(jobId)?.checkOutputs[kept] ?? refused(path(jobId, `/checks/${kept}/output`)),
    readBrief: async (jobId, name) => readsOf(jobId)?.briefs?.[name] ?? refused(path(jobId, `/briefs/${name}`)),
    readRetro: async (jobId) => {
      const retro = scenario.retros?.[jobId];
      // Each item stands as the answers left it, as Fleet's `state` says. An old item has none.
      const standing = (item: RetroItem): RetroItem => {
        const now = item.state === undefined ? undefined : lessons.find((one) => one.id === item.id);
        if (now === undefined) return item;
        return { ...item, state: now.state, ...(now.job_proposed === undefined ? {} : { job_proposed: now.job_proposed }) };
      };
      if (retro !== undefined) return { ok: true, retro: { ...retro, items: retro.items?.map(standing) } };
      return readsOf(jobId) === undefined
        ? refused(path(jobId, "/retro"))
        : { ok: true, retro: { job_id: jobId, state: "pending", record: {} } };
    },
    readLessons: async (view) => ({ ok: true, lessons: listed(lessons, view) }),
    agreeLesson: async (id) => answerLesson(id, "agree"),
    disagreeLesson: async (id) => answerLesson(id, "disagree"),
    readFrame: async (jobId, kept) => readsOf(jobId)?.frames[kept] ?? refused(path(jobId, `/frames/${kept}`)),
    readComposing: async (repository) => refused(`/composing?repository=${encodeURIComponent(repository)}`),
    // The app's own spelling, which a browser has no handler for — `props.ts`' reason.
    frameStreamUrl: (jobId, kept) => `armada-frame://frame/${jobId}/${kept}`,
    readReports: async (want) => publish({ reports: want ? unread("/reports") : nothing }),
    readHeld: async (want) => {
      // A Scout reads between two reads, as Fleet's does: each one finds it a file further on.
      if (want && held !== undefined) held = scoutRead(held);
      publish({
        held: !want ? nothing : held === undefined ? unread("/worktrees/held") : { state: "read", held },
      });
    },
    // Every scenario keeps Studios, so the surface opens wherever it is reached. A scenario naming
    // none keeps an empty list and draws its empty state, never a read failure — #1341.
    ...studios,
    approveReview: async () => OK,
    mergePullRequest: async () => OK,
    rerunFailedChecks: async () => OK,
    investigateFailedChecks: async () => OK,
    queueAfterFinding: async () => OK,
    fileFindingIssue: async () => OK,
    openFindingIssue: async () => ({ ok: false, why: "no_address" }),
    requestChanges: async () => OK,
    removeWalkNote: async () => OK,
    rejectWork: async (jobId) => (move(jobId, { status: "rejected" }), OK),
    readRemarks: async (jobId) =>
      publish({
        remarks: jobId === null ? nothing : (readsOf(jobId)?.recorded.remarks ?? failed(jobId, "/remarks")),
      }),
    takeUpRemarks: async () => OK,
    dismissFinding: async () => OK,
    openArtifact: async () => ({ ok: false, why: "no_repository" }),
    openPullRequest: async () => ({ ok: false, why: "no_address" }),
    openRemarkLink: async () => ({ ok: false, why: "no_address" }),

    // A scenario opens its Job the way a pressed notification does — once, though `StrictMode` registers twice.
    onSummoned: (onGo) => {
      summoners.add(onGo);
      if (scenario.opens !== undefined && !summoned) {
        summoned = true;
        const to = { jobId: scenario.opens };
        queueMicrotask(() => summoners.forEach((summoner) => summoner(to)));
      }
      return () => summoners.delete(onGo);
    },

    askHelm: async () => OK,
    // No session behind a mock Fleet, so the record is the refusal a window
    // with nothing connected already draws — never an invented record.
    helmDebugInfo: async () => refused("/helm/debug"),
    startHelmFresh: async () => OK,
    /**
     * The dock's own switch, and *Discuss with Helm* on a card. **Which
     * repository Helm answers for is main's decision and not Fleet's**
     * (`main/helm.ts`), so the fake takes it rather than dropping it: this was
     * a no-op, and a mock where the one control that points Helm does nothing
     * is a mock where nothing pointed can be reached at all.
     *
     * What it publishes is a conversation with nothing in it, which is what
     * main's own socket publishes on a fresh connection whose backfill found
     * none — never an invented exchange. A scenario that holds one answers
     * this itself; `helm-talking` is pointed from the start and draws no
     * switch, so nothing there passes through here.
     */
    pointHelm: async (manifestId) =>
      publish({ helm: { state: "open", manifestId, replying: false, skipped: 0, missed: 0, items: [] } }),
    // The mock provides no haptics, so nothing calls this; answered for the type.
    tap: () => undefined,
  };
  const fake = { ...api, ...scenario.behaves?.({ state: () => state, publish }) };
  DRAFTS.set(fake, {
    current: () => draft,
    subscribe: (onDraft) => {
      drafters.add(onDraft);
      return () => drafters.delete(onDraft);
    },
  });
  return fake;
}

/**
 * The row a press on Dispatch mints: the request as the title, no workflow, no
 * step and nothing that has run.
 *
 * **The manifest is the first Job's**, so the new row belongs to a repository
 * the Board is already listing rather than to one nothing serves. A scenario
 * with no Job at all mints none of its own either — there is nowhere to put it.
 */
function proposingRow(request: string, at: string, jobs: readonly JobSummary[]): JobSummary {
  const ordinal = jobs.length + 1;
  return {
    id: `01M2E0DISPATCHED${String(ordinal).padStart(2, "0")}MOCK`,
    handle: `${ordinal}-dispatched-from-the-composer`,
    title: request,
    status: "proposing",
    workflow_id: "",
    owner_manifest_id: jobs[0]?.owner_manifest_id ?? "",
    origin: "manual",
    urgency: "normal",
    atomic: false,
    model: "sonnet",
    created_at: at,
  };
}
