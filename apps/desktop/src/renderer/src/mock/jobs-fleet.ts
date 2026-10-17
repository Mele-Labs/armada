// Jobs' members as the mock Fleet answers them. Fleet still serves them; Bridge calls only the few a
// Session, the Cockpit, the Checks page and Lessons reach, and those answer from the scenario. Every
// other member answers ok or a route the mock has none for. `slices/jobs.ts` registers them.

import type { JobRetro, Lesson, LessonAnswer, Outcome, ProposalInFlight, RetroItem } from "@armada/protocol";
import type { JobFixture } from "@armada/screens/src/fixtures/fixture";

import type { JobsApi } from "../../../shared/api/jobs";
import { PROPOSER_BUDGET_MS } from "@armada/screens/src/fixtures/build/proposing";
import { READ_MS, requestRepositoryRead } from "./request-repository-fleet";
import { answered, askedAbout, listed, reviewed, sessionStarted } from "@armada/screens/src/lessons/lessons-fleet";
import type { Fleet } from "./fake-context";
import { nothing, proposingRow } from "./fake-context";

/** What a scenario holds that Jobs' members answer from: each Job's reads, retros and Lessons. */
export type JobsScenario = {
  reads: Record<string, JobFixture>;
  retros?: Record<string, JobRetro>;
  lessons?: Lesson[];
};

const OK: Outcome = { ok: true };
/** The paths the mock's `@` can find, narrowed by what follows it as Fleet's search narrows them. */
const FILES = [
  "README.md",
  "crates/fleet/src/sessions.rs",
  "docs/practices/bridge.md",
  "docs/practices/session-composer.md",
  "packages/components/src/compositions/SessionComposer/SessionComposer.tsx",
  "packages/components/src/compositions/SessionComposer/InlineTag.tsx",
  "apps/desktop/src/renderer/src/sessions.tsx",
  "apps/desktop/src/renderer/src/mock/sessions/script.ts",
];

export const jobsApi = (scenario: JobsScenario, fleet: Fleet): JobsApi => {
    const { state, publish, readsOf, path, failed, refused } = fleet;

    /** The retro items as answers left them. */
    let lessons = scenario.lessons ?? [];

    /** One answer to one item. An agreed Armada or Manifest item puts its Job on the Board at the gate. */
    function answerLesson(id: string, answer: "agree" | "disagree"): LessonAnswer {
      const taken = answered(lessons, id, answer, state().jobs, new Date().toISOString());
      lessons = taken.lessons;
      // Update Kit adds the command Fleet says it applied, as that retro item's.
      const applied = taken.answer.ok ? taken.answer.lesson.applied : undefined;
      if (applied !== undefined && !fleet.kit.get().some((one) => one.run === applied.command)) {
        fleet.kit.set([...fleet.kit.get(), { run: applied.command, source: "retro_item", lesson_id: id }]);
      }
      if (taken.job !== undefined) publish({ jobs: [...state().jobs, taken.job] });
      return taken.answer;
    }

    return {
      // **The row appears and the call never answers.** A dispatched request is a
      // Job from the press — `job-statuses.toml`, `proposing` — and that Job is
      // the only thing the mock can honestly mint. The promise is left out because
      // the proposer never answers in the mock; nothing is waiting on it, since the
      // press left the composer.
      proposeFromRequest: (request, _attachments, repository) => {
        const at = new Date().toISOString();
        // The repository the dispatch named, by its Manifest: the Job lands on the one chosen, not on the first the Board lists.
        const named = repository == null ? undefined : state().holds.repositories?.find((one) => one.root === repository)?.manifest?.id;
        const row = proposingRow(request, at, state().jobs, named);
        // The call is named with the Job it is reading for, as main's arrival carries it, so the
        // window that dispatched learns the Job before anything answers.
        const proposing: ProposalInFlight = {
          proposal_id: `01M2E0PROPOSAL${row.id.slice(-8)}`,
          job_id: row.id,
          model: row.model,
          since: at,
          budget_ms: PROPOSER_BUDGET_MS,
          reached: "starting",
        };
        publish({ jobs: [...state().jobs, row], proposing });
        return new Promise(() => {});
      },
      stageAttachment: async (_bytes, filename) => ({ path: filename }),
      searchFiles: async (query) => FILES.filter((path) => path.toLowerCase().includes(query.toLowerCase())),
      readRequestRepository: async (words) => {
        await new Promise((resume) => setTimeout(resume, READ_MS));
        return requestRepositoryRead(words, state().holds.repositories ?? [], state().jobs);
      },
      answerQuestion: async () => OK,
      answerCommand: async () => OK,
      answerJudge: async () => OK,
      parkJob: async () => OK,
      resumeJob: async () => OK,

      // Routes Bridge no longer calls and Fleet still serves: accepted, or refused as a route the mock has none for.
      stopProposal: async () => OK,
      approveDispatch: async () => OK,
      toProposer: async () => OK,
      setLandingTarget: async () => OK,
      listBranches: async (manifestId) => refused(`/manifest/branches?manifest_id=${encodeURIComponent(manifestId)}`),
      redispatchJob: async () => OK,
      killDrone: async () => OK,
      killJob: async () => OK,
      fixMain: async () => OK,
      killProcess: async () => OK,
      killProcesses: async () => OK,
      pilotTask: async (jobId, taskId) => refused(path(jobId, `/tasks/${taskId}/pilot`)).outcome,
      restartTask: async () => OK,
      editTask: async () => OK,
      movePlan: async () => OK,
      approveWave: async () => OK,
      editJob: async () => OK,
      redirectDrone: async () => OK,
      explainCommand: async (jobId, callId) => refused(path(jobId, `/calls/${callId}/explain`)),
      setWhenBlocked: async () => OK,
      setWhenRefused: async () => OK,
      setModel: async () => OK,
      setReviewModel: async () => OK,
      removeAllowedCommand: async () => OK,
      restartStep: async () => OK,
      overrideVerdict: async () => OK,
      rerunGate: async () => OK,
      rerunChecks: async () => OK,
      retryCheck: async () => OK,
      skipCheck: async () => OK,
      skipChecks: async () => OK,
      showAgain: async () => OK,
      raiseCostCap: async () => OK,
      raiseTurnCap: async () => OK,
      askCommander: async () => OK,
      answerCommanderCard: async () => OK,
      addTask: async (jobId) => refused(path(jobId, "/add_task")),
      dropTask: async (jobId) => refused(path(jobId, "/drop_task")),
      watchJob: async (jobId) =>
        publish({ watched: jobId === null ? nothing : failed(jobId, ""), jobDrones: nothing, footprint: nothing, handed: nothing }),
      observeJob: async (jobId) => publish({ observed: jobId === null ? nothing : (readsOf(jobId)?.observed ?? nothing) }),
      followCheckOutput: async () => publish({ followed: nothing }),
      followLandCheck: async () => publish({ landFollowed: nothing }),
      readHistory: async () => publish({ history: nothing }),
      followActive: async () => {},
      readResources: async () => publish({ resources: nothing }),
      watchPulse: async () => {},
      watchRunSheet: async (jobId) => publish({ runSheet: jobId === null ? nothing : failed(jobId, "/runs/sheet") }),
      observeRun: async () => publish({ runFollowed: nothing }),
      startRun: async () => OK,
      stopRun: async () => OK,
      undoRun: async () => OK,
      listRuns: async (jobId) => refused(path(jobId, "/runs")),
      getRunOutput: async (jobId, runId) => refused(path(jobId, `/runs/${runId}/output`)),

      examineJob: async (jobId) => publish({ examination: failed(jobId, "/examine") }),
      readEvidence: async () => publish({ evidence: nothing }),
      readDiff: async () => publish({ diff: nothing }),
      readCheckOutput: async (jobId, kept) =>
        readsOf(jobId)?.checkOutputs[kept] ?? refused(path(jobId, `/checks/${kept}/output`)),
      readSessionCheckOutput: async (run) => refused(`/sessions/checks/${run}/output`),
      stopSessionCheck: async () => ({ ok: true }),
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
      startLessonSession: async (id) => {
        const taken = sessionStarted(lessons, id);
        lessons = taken.lessons;
        return taken.read;
      },
      reviewLessons: async () => reviewed(lessons),
      askLesson: async (id, question, history) => askedAbout(lessons, id, { question, history }),
      readFrame: async (jobId, kept) => readsOf(jobId)?.frames[kept] ?? refused(path(jobId, `/frames/${kept}`)),
      readComposing: async (repository) => refused(`/composing?repository=${encodeURIComponent(repository)}`),
      frameStreamUrl: (jobId, kept) => `armada-frame://frame/${jobId}/${kept}`,
      approveReview: async () => OK,
      mergePullRequest: async () => OK,
      autoMergePullRequest: async (jobId) => refused(path(jobId, "auto_merge")).outcome,
      rerunFailedChecks: async () => OK,
      investigateFailedChecks: async () => OK,
      queueAfterFinding: async () => OK,
      fileFindingIssue: async () => OK,
      openFindingIssue: async () => ({ ok: false, why: "no_address" }),
      requestChanges: async () => OK,
      removeWalkNote: async () => OK,
      rejectWork: async () => OK,
      readRemarks: async () => publish({ remarks: nothing }),
      takeUpRemarks: async () => OK,
      dismissFinding: async () => OK,
      openArtifact: async () => ({ ok: false, why: "no_repository" }),
      openInTerminal: async () => ({ ok: false, why: "no_repository" }),
      openPullRequest: async () => ({ ok: false, why: "no_address" }),
      openRemarkLink: async () => ({ ok: false, why: "no_address" }),
    };
};
