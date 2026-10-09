// Jobs' members as a mock Fleet answers them: dispatching and approving, the open Job's reads, plan
// edits, review, retros and Lessons. Desktop's `mock/slices/jobs.ts` registers them over its fleet.

import { unanswered } from "@armada/bridge-api";
import type { FleetHandle } from "@armada/bridge-api";
import { refusedWith } from "@armada/protocol";
import type {
  HeldWorktrees,
  JobDetail,
  JobRetro,
  JobSummary,
  KitAllowedCommand,
  Lesson,
  LessonAnswer,
  Outcome,
  RetroItem,
  WorkPlan,
  WorktreesHeld,
} from "@armada/protocol";
import { autoMergeRefusal, pullRequestStateOf } from "./pull-request-fleet";
import type { JobFixture } from "@armada/screens/src/fixtures/fixture";
import type { PlanEditAnswer } from "@armada/screens/src/plan-edits";

import type { JobsApi, JobsState } from "../api";
import type { ArcDraft } from "../fixtures/build/arc";
import type { GroupView } from "../draft/group";
import { isOutcome, parked, pausedRefusal, resumed } from "./pause-fleet";
import { accepted, askedAgain } from "./undecided-fleet";
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

/** What a read with nothing behind it holds. */
const nothing = { state: "none" } as const;

/** The state Jobs' members read and write: the Board's Jobs, and what the pool holds. */
export type JobsHeld = JobsState & { jobs: JobSummary[]; held: HeldWorktrees };

/** What a scenario holds that Jobs' members answer from: each Job's reads, retros and Lessons. */
export type JobsScenario = {
  reads: Record<string, JobFixture>;
  retros?: Record<string, JobRetro>;
  lessons?: Lesson[];
};

/**
 * What Jobs' members reach beyond the state: one field on one Job, the plan and its draft, the pool,
 * Kit's allowlist, the reads a Job holds, and the routes a read answers refused. The app's fake supplies them.
 */
export type JobsFleet = FleetHandle<JobsHeld> & {
  move: (jobId: string, change: Partial<JobSummary>) => void;
  stepping: (jobId: string, moved: (whole: JobDetail, at: string) => JobDetail | undefined) => void;
  editPlan: (
    jobId: string,
    route: string,
    plan: (was: WorkPlan) => WorkPlan | undefined,
    groups: (was: GroupView[]) => GroupView[],
  ) => PlanEditAnswer;
  held: { get: () => WorktreesHeld | undefined; set: (held: WorktreesHeld | undefined) => void };
  draft: { get: () => ArcDraft | undefined; set: (draft: ArcDraft | undefined) => void };
  kit: { get: () => KitAllowedCommand[]; set: (allowed: KitAllowedCommand[]) => void };
  readsOf: (jobId: string) => JobFixture | undefined;
  path: (jobId: string, route?: string) => string;
  failed: (jobId: string, route: string) => { state: "failed"; jobId: string; outcome: Outcome };
  refused: (route: string) => { ok: false; outcome: Outcome };
  proposingRow: (request: string, at: string, jobs: readonly JobSummary[]) => JobSummary;
};

const OK: Outcome = { ok: true };
/**
 * How far apart a fixture's `arriving` rows land. Long enough that a walk has
 * opened a transcript's panel before the first one, short enough that the
 * walk's five-second wait for it holds.
 */
const ARRIVING_MS = 4000;

export const jobsApi = (scenario: JobsScenario, fleet: JobsFleet): JobsApi => {
    const { state, publish, move, stepping, editPlan, held: pool, draft, kit, readsOf, path, failed, refused, proposingRow } = fleet;

    /** The retro items as answers left them. */
    let lessons = scenario.lessons ?? [];

    /** One answer to one item. An agreed Armada or Manifest item puts its Job on the Board at the gate. */
    function answerLesson(id: string, answer: "agree" | "disagree"): LessonAnswer {
      const taken = answered(lessons, id, answer, state().jobs, new Date().toISOString());
      lessons = taken.lessons;
      // Update Kit adds the command Fleet says it applied, as that retro item's.
      const applied = taken.answer.ok ? taken.answer.lesson.applied : undefined;
      if (applied !== undefined && !kit.get().some((one) => one.run === applied.command)) {
        kit.set([...kit.get(), { run: applied.command, source: "retro_item", lesson_id: id }]);
      }
      if (taken.job !== undefined) publish({ jobs: [...state().jobs, taken.job] });
      return taken.answer;
    }

    /** A Job paused or resumed, folded as `job.paused` and `job.resumed` fold: the row whole, and the pool read again. */
    function pausing(jobId: string, kind: "job.paused" | "job.resumed"): Outcome {
      const was = state().jobs.find((job) => job.id === jobId);
      if (was === undefined) return unanswered(path(jobId));
      const held = pool.get();
      const done = kind === "job.paused" ? parked(was, held, new Date().toISOString()) : resumed(was, held);
      if (isOutcome(done)) return done;
      pool.set(done.held);
      const watched = state().watched;
      publish({
        jobs: state().jobs.map((job) => (job.id === jobId ? done.job : job)),
        watched: watched.state === "read" && watched.jobId === jobId ? { ...watched, detail: { ...watched.detail, job: done.job } } : watched,
        ...(state().held.state === "read" && done.held !== undefined ? { held: { state: "read", held: done.held } } : {}),
      });
      return OK;
    }

    /** An act on a paused Job, refused as Fleet refuses it; `undefined` where it goes on. */
    const whilePaused = (jobId: string) => pausedRefusal(state().jobs.find((job) => job.id === jobId));

    return {
      // **The row appears and the call never answers.** A dispatched request is a
      // Job from the press — `job-statuses.toml`, `proposing` — and that Job is
      // the only thing the mock can honestly mint: no workflow, no steps and no
      // plan, so there is nothing here to invent. The promise is left out because
      // the proposer never answers in the mock, which is what `in_code = "Not
      // yet"` means; nothing is waiting on it, since the press left the composer.
      proposeFromRequest: (request) => {
        const at = new Date().toISOString();
        const row = proposingRow(request, at, state().jobs);
        publish({ jobs: [...state().jobs, row] });
        return new Promise(() => {});
      },
      stopProposal: async () => OK,
      stageAttachment: async (_bytes, filename) => ({ path: filename }),
      searchFiles: async () => [],

      // The body applied as Fleet applies it (23.8), on the Job open; a row not
      // open only moves to `queued`, which is all the Board draws of it.
      approveDispatch: async (jobId, approval) => {
        const watched = state().watched;
        if (watched.state !== "read" || watched.jobId !== jobId) return (move(jobId, { status: "queued" }), OK);
        const refusal = tuningRefusal(watched.detail, approval) ?? landingRefusal(approval?.landing);
        if (refusal !== undefined) return refusal;
        const detail = approvedAs(watched.detail, approval, new Date().toISOString());
        publish({
          jobs: state().jobs.map((job) => (job.id === jobId ? { ...job, ...detail.job } : job)),
          watched: { ...watched, detail },
        });
        return OK;
      },
      // The proposal back to the proposer with a note (`to_proposer`, 23.25).
      toProposer: async (jobId, body) => {
        const watched = state().watched;
        if (watched.state !== "read" || watched.jobId !== jobId) return OK;
        const back = sentBack(watched.detail, body);
        if ("ok" in back) return back;
        publish({ watched: { ...watched, detail: back } });
        return OK;
      },
      // Where an approved Job lands, once (`set_landing_target`, 23.22).
      setLandingTarget: async (jobId, target) => {
        const watched = state().watched;
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
      parkJob: async (jobId) => pausing(jobId, "job.paused"),
      resumeJob: async (jobId) => pausing(jobId, "job.resumed"),
      fixMain: async () => OK,
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
          draft.get()?.groups?.some((group) => group.tasks.some((task) => task.id === taskId && task.state === "failed")) ??
          false;
        const open = state().watched;
        const answer = editPlan(
          jobId,
          `/tasks/${taskId}/restart`,
          (was) => planRestarting(was, taskId, failedInDraft, open.state === "read" ? open.detail.steps : []),
          (was) => groupsRestarting(was, taskId),
        );
        return answer.ok ? OK : answer.outcome;
      },
      // Served since 23.6 (#1657): the fields sent, over the task's own.
      editTask: async (jobId, taskId, edit) => {
        const editableInDraft =
          draft.get()?.groups?.some((group) =>
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
      movePlan: async (jobId, change) => {
        const answer = editPlan(
          jobId,
          "/plan/move",
          (was) => planMoving(was, change),
          (was) => groupsMoving(was, change),
        );
        return answer.ok ? OK : answer.outcome;
      },
      // An Epic's plan approved with its wave, served since 23.11 (#1694): every
      // Job named leaves its gate, and the Epic is queued behind them.
      approveWave: async (jobId, wave) => {
        wave.jobs.forEach((one) => move(one, { status: "queued" }));
        move(jobId, { status: "queued" });
        const was = draft.get();
        const drawn = was?.wave;
        if (drawn !== undefined) {
          const released = (one: (typeof drawn.jobs)[number]) =>
            wave.jobs.includes(one.job) ? { ...one, status: "queued" } : one;
          draft.set({ ...was, wave: { ...drawn, jobs: drawn.jobs.map(released) } });
        }
        return OK;
      },
      // A proposal's words saved without releasing it (`edit_job`, 23.8): on the
      // row, on the Job where it is open, and on the wave's panel that sent it.
      editJob: async (jobId, edit) => {
        if (edit.title !== undefined) move(jobId, { title: edit.title });
        const watched = state().watched;
        if (watched.state === "read" && watched.jobId === jobId) {
          publish({ watched: { ...watched, detail: edited(watched.detail, edit) } });
        }
        const was = draft.get();
        const wave = was?.wave;
        if (wave !== undefined) {
          draft.set({ ...was, wave: { ...wave, jobs: wave.jobs.map((one) => (one.job === jobId ? waveJobEdited(one, edit) : one)) } });
        }
        return OK;
      },
      redirectDrone: async () => OK,
      answerQuestion: async () => OK,
      answerCommand: async (_jobId, _call, answer, _note, rule, scope) => {
        // Always allow on this machine keeps the rule it was sent with in Kit's allowlist.
        if (answer === "always_allow" && scope === "kit" && rule !== undefined && !kit.get().some((one) => one.run === rule)) {
          kit.set([...kit.get(), { run: rule, source: "always_allow" }]);
        }
        return OK;
      },
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
      restartStep: async (jobId) => whilePaused(jobId) ?? OK,
      overrideVerdict: async (jobId) => whilePaused(jobId) ?? (stepping(jobId, accepted), OK),
      rerunGate: async (jobId) => (stepping(jobId, askedAgain), OK),
      rerunChecks: async () => OK,
      showAgain: async () => OK,
      raiseCostCap: async () => OK,
      raiseTurnCap: async () => OK,
      addTask: async (jobId, add) => {
        let id = "";
        return editPlan(
          jobId,
          "/add_task",
          (was) => ((id = nextTaskId(was, draft.get()?.groups)), planAdding(was, id, add)),
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
        // The Job as the board holds it now, so a pause made before it was opened is on its detail.
        const row = state().jobs.find((job) => job.id === jobId);
        const watched = reads?.watched;
        publish({
          watched:
            jobId === null
              ? nothing
              : watched?.state === "read" && row !== undefined
                ? { ...watched, detail: { ...watched.detail, job: row } }
                : (watched ?? failed(jobId, "")),
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
            const now = state().observed;
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

      examineJob: async (jobId) => publish({ examination: failed(jobId, "/examine") }),
      readEvidence: async (jobId) =>
        publish({
          evidence: jobId === null ? nothing : (readsOf(jobId)?.recorded.evidence ?? failed(jobId, "/evidence")),
        }),
      readDiff: async (jobId) =>
        publish({ diff: jobId === null ? nothing : (readsOf(jobId)?.recorded.diff ?? failed(jobId, "/diff")) }),
      readCheckOutput: async (jobId, kept) =>
        readsOf(jobId)?.checkOutputs[kept] ?? refused(path(jobId, `/checks/${kept}/output`)),
      readSessionCheckOutput: async (run) => refused(`/sessions/checks/${run}/output`),
      readBrief: async (jobId, name) => readsOf(jobId)?.briefs?.[name] ?? refused(path(jobId, `/briefs/${name}`)),
      readRetro: async (subject) => {
        const jobId = subject.id;
        // A Session's numbered retro is held under `{session}-r{n}`; its newest under the Session's id. An unknown `n` is refused.
        const retro = scenario.retros?.[subject.kind === "session" && subject.n !== undefined ? `${jobId}-r${subject.n}` : jobId];
        if (subject.kind === "session" && retro === undefined) return refused(`/sessions/${jobId}/retro${subject.n === undefined ? "" : `?n=${subject.n}`}`);
        // Each item stands as the answers left it, as Fleet's `state` says. An old item has none.
        const standing = (item: RetroItem): RetroItem => {
          const now = item.state === undefined ? undefined : lessons.find((one) => one.id === item.id);
          if (now === undefined) return item;
          return {
            ...item,
            state: now.state,
            ...(now.job_proposed === undefined ? {} : { job_proposed: now.job_proposed }),
            ...(now.applied === undefined ? {} : { applied: now.applied }),
          };
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
      approveReview: async (jobId) => whilePaused(jobId) ?? OK,
      // The forge merged it and Fleet took the work: the row completes and the pull request reads merged.
      mergePullRequest: async (jobId) => {
        // A frozen repository takes the press and carries it out when the freeze lifts.
        if ((state().jobs.find((job) => job.id === jobId)?.frozen_by ?? []).length > 0) return OK;
        const watched = state().watched;
        const done = { status: "completed_success", landed: "merged" } as const;
        publish({
          jobs: state().jobs.map((job) => (job.id === jobId ? { ...job, ...done } : job)),
          ...(watched.state === "read" && watched.jobId === jobId
            ? { watched: { ...watched, detail: { ...watched.detail, job: { ...watched.detail.job, ...done }, delivery: { ...watched.detail.delivery, landed: "merged" } } } }
            : {}),
        });
        return OK;
      },
      autoMergePullRequest: async (jobId) => {
        const watched = state().watched;
        if (watched.state !== "read" || watched.jobId !== jobId) return OK;
        const reading = pullRequestStateOf(watched.detail, true);
        if (reading === undefined) return unanswered(path(jobId));
        const why = autoMergeRefusal(reading);
        if (why !== undefined) {
          return refusedWith(409, JSON.stringify({ code: "fleet.merge_checks_not_passed", message: why }), {
            method: "POST",
            path: path(jobId, "auto_merge"),
          });
        }
        return { ok: true, pullRequest: reading };
      },
      rerunFailedChecks: async () => OK,
      investigateFailedChecks: async () => OK,
      queueAfterFinding: async () => OK,
      fileFindingIssue: async () => OK,
      openFindingIssue: async () => ({ ok: false, why: "no_address" }),
      requestChanges: async (jobId) => whilePaused(jobId) ?? OK,
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
    };
};
