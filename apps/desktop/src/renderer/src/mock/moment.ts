// What every scenario is made of: the state a connected Fleet publishes, and a
// scenario built from rows a test picks. Apart from `scenario.ts`, so a
// scenario's own Fleet (`setup-fleet.ts`) can build on it and still be listed
// there without the two importing each other. The generic parts live in
// `@armada/bridge-api`; this file fixes them to desktop's `BridgeState` and `BridgeApi`.

import { connected as connectedOver, onBoard as onBoardOver, unanswered } from "@armada/bridge-api";
import type { FleetHandle as FleetHandleOf, Scenario as ScenarioOf } from "@armada/bridge-api";
import type { ArcDraft } from "@armada/jobs/fixtures/build/arc";
import type { JobSummary, RepositorySummary, WorkflowSummary } from "@armada/protocol";

import type { Session } from "@armada/screens/src/draft/sessions";
import type { TimelineEvent, WriteClash } from "@armada/protocol";
import type { RegisteredOwnerRow, SharedRow, SyncItem, SyncKindWord } from "@armada/components";
import type { AreaSource } from "../cockpit/deck/areas";
import type { ProgressScript } from "./progress";
import type { CommanderStore } from "./commander-fleet";
import type { Fleet } from "./fake-context";
import type { BoardControl, SessionsStore } from "./sessions/script";

import type { BridgeApi } from "../../../shared/api";
import { NOTHING_YET } from "../../../shared/bridge";
import type { BridgeState } from "../../../shared/bridge";

export { unanswered };

/** One moment: what is published before anything is opened, and the reads behind each Job. */
/**
 * What a moment holds that Fleet cannot serve yet: the Arc milestone's draft, and Sessions.
 * **Sessions are built per window** (a factory), because a Session's turns are time in this window.
 */
/** `more` is Sessions open beside the usual ones, for a scenario that wants others. */
export type MockDraft = ArcDraft & {
  sessions?: (board: BoardControl, more?: readonly Session[]) => SessionsStore;
  /** A Job's Commander, built per window over its Fleet: its thread, and what it changes on the Job. */
  commanding?: (fleet: Fleet) => CommanderStore;
  /** What Fleet did, for the Cockpit's timeline: no wire serves it yet. */
  events?: readonly TimelineEvent[];
  /** The write clashes Fleet reports, as `GET /clashes` returns them. Mock only. */
  clashes?: readonly WriteClash[];
  /** The directories a repository is drawn in on the flight deck's map: Fleet reports none yet, so the real source is empty. Mock only. */
  areas?: AreaSource;
  /** What each Session has built, moment by moment: Fleet serves no task list yet. Mock only. */
  progress?: ProgressScript;
  /** The shared home this machine is signed in to, and how each row stands against it. Mock only. */
  sharedHome?: { account: string; repository: string; rows: readonly SharedRow[] };
  /**
   * The machine starts signed out. Signing in as `signIn.login` shows `signIn.code` and sets up the
   * shared home `repository`; `add` is the second login a profile adds, with the items each of its
   * kinds would sync. Mock only.
   */
  account?: {
    signIn: { login: string; code: string; repository: string };
    add?: { login: string; code: string; items: Partial<Record<SyncKindWord, readonly SyncItem[]>> };
  };
  /**
   * The owners registered to pull requests, as the central store holds them, and which repositories
   * register at all, by root and by `owner/name`. Mock only: Fleet serves neither.
   */
  prOwners?: {
    repositories: readonly { root: string; name: string; registering: boolean }[];
    rows: readonly { repository: string; number: number; owner: RegisteredOwnerRow }[];
  };
};

export type Scenario = ScenarioOf<BridgeState, BridgeApi, MockDraft>;

/** What a scenario's `behaves` reaches: the state as published, and the one way to change it. */
export type FleetHandle = FleetHandleOf<BridgeState>;

/** The state a connected Fleet publishes, holding these Jobs. */
export function connected(
  jobs: JobSummary[],
  workflows: WorkflowSummary[],
  repositories: RepositorySummary[],
): BridgeState {
  return connectedOver(NOTHING_YET, jobs, workflows, repositories);
}

/** A connected Fleet holding exactly these rows, for a test that needs a Board no named scenario draws. */
export function onBoard(
  jobs: JobSummary[],
  options: { workflows?: WorkflowSummary[]; repositories?: RepositorySummary[]; picked?: string | null } = {},
): Scenario {
  return onBoardOver<BridgeState, BridgeApi>(NOTHING_YET, jobs, options);
}
