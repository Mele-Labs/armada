// What a fake's slices share: the state it publishes, the one way to change it, and the few
// things more than one surface reads or writes. Built once per window by `fake.ts`.

import type { JobDetail, JobSummary, KitAllowedCommand, WorkPlan, WorktreesHeld } from "@armada/protocol";
import type { ArcDraft } from "@armada/jobs/fixtures/build/arc";
import type { GroupView } from "@armada/jobs/draft/group";
import type { PlanEditAnswer } from "@armada/screens/src/plan-edits";

import type { BridgeState } from "../../../shared/bridge";
import { unanswered } from "./moment";
import type { Scenario } from "./moment";

export type Fleet = {
  scenario: Scenario;
  state: () => BridgeState;
  /** Publish a change — on a microtask, because main's reach a window over IPC, never inside the call. */
  publish: (change: Partial<BridgeState>) => void;
  /** Hear every published state. */
  listen: (onState: (state: BridgeState) => void) => () => void;
  /** One field on one Job, and on its detail where that Job is the one open. */
  move: (jobId: string, change: Partial<JobSummary>) => void;
  /** A Job the Judge did not answer on, moved the way Fleet would; any other is left as drawn. */
  stepping: (jobId: string, moved: (whole: JobDetail, at: string) => JobDetail | undefined) => void;
  /** A person's own add or drop, on the open plan and on the draft groups an arc board draws. */
  editPlan: (
    jobId: string,
    route: string,
    plan: (was: WorkPlan) => WorkPlan | undefined,
    groups: (was: GroupView[]) => GroupView[],
  ) => PlanEditAnswer;
  /** The pool as acts on it left it, read again after each as main does. */
  held: { get: () => WorktreesHeld | undefined; set: (held: WorktreesHeld | undefined) => void };
  /** The draft as it stands; `set` tells whoever is drawing it. */
  draft: { get: () => ArcDraft | undefined; set: (draft: ArcDraft | undefined) => void; subscribe: (onDraft: () => void) => () => void };
  /** What Kit's allowlist holds on this machine. */
  kit: { get: () => KitAllowedCommand[]; set: (allowed: KitAllowedCommand[]) => void };
  readsOf: (jobId: string) => Scenario["reads"][string] | undefined;
  path: (jobId: string, route?: string) => string;
  failed: (jobId: string, route: string) => { state: "failed"; jobId: string; outcome: ReturnType<typeof unanswered> };
  refused: (route: string) => { ok: false; outcome: ReturnType<typeof unanswered> };
  unread: (route: string) => { state: "failed"; outcome: ReturnType<typeof unanswered> };
};

/** The surfaces a fake is composed of, one `shared/api/<name>.ts` each. */
export type SliceName =
  | "core"
  | "studios"
  | "manifest"
  | "setup"
  | "workflows"
  | "triggers"
  | "helm"
  | "settings"
  | "overview"
  | "cleanup"
  | "reports"
  | "jobs"
  | "sessions";

/**
 * One surface's part of a fake. `api` is the members its `shared/api/<name>.ts` declares, answered
 * from the scenario; `state` is what that surface's fields hold when nothing has read them yet,
 * which a window mounted without the slice keeps. `scenarios` are the moments only this surface's
 * Fleet makes, listed by `scenario.ts` where it lists the rest.
 */
export type Slice<A, S> = {
  name: SliceName;
  api: (scenario: Scenario, fleet: Fleet) => A;
  state: S;
  scenarios?: () => Scenario[];
};

/** The fixed answers a read with nothing behind it gives. */
export const nothing = { state: "none" } as const;

/** One window's fleet, over `initial` as the state it opens on. */
export function fleetOf(scenario: Scenario, initial: BridgeState): Fleet {
  let state = initial;
  const listeners = new Set<(state: BridgeState) => void>();
  const drafters = new Set<() => void>();
  let draft = scenario.draft;
  let held = scenario.held;
  let kit: KitAllowedCommand[] = [];

  function publish(change: Partial<BridgeState>): void {
    state = { ...state, ...change };
    const now = state;
    queueMicrotask(() => listeners.forEach((listener) => listener(now)));
  }

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

  function stepping(jobId: string, moved: (whole: JobDetail, at: string) => JobDetail | undefined): void {
    const watched = state.watched;
    if (watched.state !== "read" || watched.jobId !== jobId) return;
    const detail = moved(watched.detail, new Date().toISOString());
    if (detail === undefined) return;
    publish({ jobs: state.jobs.map((job) => (job.id === jobId ? detail.job : job)), watched: { ...watched, detail } });
  }

  const path = (jobId: string, route = "") => `/jobs/${encodeURIComponent(jobId)}${route}`;
  const refused = (route: string) => ({ ok: false, outcome: unanswered(route) }) as const;

  /**
   * **Refused where no plan is open to change**, as Fleet refuses a Job with no plan recorded.
   * Folded where main's `foldPlan` folds the plan Fleet answers with.
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

  return {
    scenario,
    state: () => state,
    publish,
    listen: (onState) => {
      listeners.add(onState);
      return () => listeners.delete(onState);
    },
    move,
    stepping,
    editPlan,
    held: { get: () => held, set: (next) => void (held = next) },
    draft: {
      get: () => draft,
      set: (next) => {
        draft = next;
        queueMicrotask(() => drafters.forEach((onDraft) => onDraft()));
      },
      subscribe: (onDraft) => {
        drafters.add(onDraft);
        return () => drafters.delete(onDraft);
      },
    },
    kit: { get: () => kit, set: (next) => void (kit = next) },
    readsOf: (jobId) => scenario.reads[jobId],
    path,
    failed: (jobId, route) => ({ state: "failed", jobId, outcome: unanswered(path(jobId, route)) }) as const,
    refused,
    unread: (route) => ({ state: "failed", outcome: unanswered(route) }) as const,
  };
}

/**
 * The row a press on Dispatch mints: the request as the title, no workflow, no
 * step and nothing that has run.
 *
 * **The manifest is the first Job's**, so the new row belongs to a repository
 * the Board is already listing rather than to one nothing serves. A scenario
 * with no Job at all mints none of its own either — there is nowhere to put it.
 */
export function proposingRow(request: string, at: string, jobs: readonly JobSummary[]): JobSummary {
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
