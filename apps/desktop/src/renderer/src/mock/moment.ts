// What every scenario is made of: the state a connected Fleet publishes, and a
// scenario built from rows a test picks. Apart from `scenario.ts`, so a
// scenario's own Fleet (`setup-fleet.ts`) can build on it and still be listed
// there without the two importing each other. The generic parts live in
// `@armada/bridge-api`; this file fixes them to desktop's `BridgeState` and `BridgeApi`.

import { connected as connectedOver, onBoard as onBoardOver, unanswered } from "@armada/bridge-api";
import type { FleetHandle as FleetHandleOf, Scenario as ScenarioOf } from "@armada/bridge-api";
import type { RepositorySummary, WorkflowSummary } from "@armada/protocol";

import type { Session } from "@armada/screens/src/draft/sessions";
import type { RunFact, TimelineEvent, WriteClash } from "@armada/protocol";
import type { BlankSetting, EndedOutcome, RegisteredOwnerRow, SharedRow, SyncItem, SyncKindWord, Syncs } from "@armada/components";
import type { AreaSource } from "../cockpit/deck/areas";
import type { ReviewSeed } from "./review-seed";
import type { PanelReads } from "../shared-panel/reads";
import type { PullRequestPages } from "./session-pr-pages";
import type { BayHistory } from "@armada/protocol";
import type { FlightControls } from "@armada/protocol";
import type { BayControl, SessionsStore } from "./sessions/script";
import type { RequestsSeed } from "./slices/requests";
import type { IssuesSeed } from "./slices/issues";
import type { ProjectsSeed } from "./slices/projects";
import type { LaterBatch } from "./events-later";

import type { BridgeApi } from "../../../shared/api";
import { NOTHING_YET } from "../../../shared/bridge";
import type { BridgeState } from "../../../shared/bridge";

export { unanswered };

/**
 * What a moment holds that Fleet cannot serve yet: Sessions, and the mock-only reads below.
 * **Sessions are built per window** (a factory), because a Session's turns are time in this window.
 * `more` is Sessions open beside the usual ones, for a scenario that wants others.
 */
export type MockDraft = {
  sessions?: (board: BayControl, more?: readonly Session[]) => SessionsStore;
  /** The Workspace the Workflows page draws as lanes, and the changes an agent comes back with for it. The page's own mock: with it the page does not read what Fleet keeps (`pieces`). */
  workspace?: import("../workspace-draft").WorkspaceSeed;
  /** The Workspaces, lifecycles and rules the mock Fleet keeps, which the lifecycle reads answer and a save changes. The page draws them as Fleet's, and a scenario with `workspace` as well is drawn from that. */
  pieces?: import("./pieces-seed").PiecesSeed;
  /** What Fleet did, for the Cockpit's timeline: no wire serves it yet. */
  events?: readonly TimelineEvent[];
  /** Entries added to the timeline as each walk `later` step lets time pass, one batch a step, each built then so it is dated then, or time passing over those held (`events-later.ts`). Mock only. */
  eventsLater?: readonly (() => LaterBatch)[];
  /** The write clashes Fleet reports, as `GET /clashes` returns them. Mock only. */
  clashes?: readonly WriteClash[];
  /** Each bay's leases and checkouts, as `getSlotHistory` returns them, newest first. Mock only. */
  bayHistories?: readonly BayHistory[];
  /** How every repository runs, as `get_run_memory` lists it: the onboarding slice answers it, and keeps what is dropped and where it is kept. Absent is the tests and the dev server. */
  runFacts?: readonly RunFact[];
  /** The hosts holding a cookie, the saved environments and each Job's servers, and how a send is answered: Requests' slice reads them. Mock only. */
  requests?: RequestsSeed;
  /** Each repository's own `.armada/flight-controls.toml` as committed, by root: the flight controls slice answers it as `in_repository`, and asks before a shell tile's first run. Mock only. */
  committedTiles?: Readonly<Record<string, FlightControls>>;
  /** The actions the person keeps for a repository, by root, as the window opens. Mock only. */
  repositoryTiles?: Readonly<Record<string, FlightControls>>;
  /** The directories a repository is drawn in on the flight deck's map: Fleet reports none yet, so the real source is empty. Mock only. */
  areas?: AreaSource;
  /** The pull requests waiting on him and what an agent review of each would find: the review surface's seed, built into a store per window. Mock only. */
  prReview?: ReviewSeed;
  /** Each repository's tracker for the Issues tab, its issues and what the model writes, built into a store per window. Mock only: no route serves the tab yet. */
  issuesTab?: IssuesSeed;
  /** The bugs agents reported in Armada itself, which the Issues tab's band and the Cockpit's marks read. The mock Fleet answers the bug report routes from it (`slices/bug-reports.ts`). */
  reportedBugs?: readonly import("../issues-tab/model").ReportedBug[];
  /** Each repository's projects as Fleet lists them, which the mock Fleet answers the Projects routes from. A window with none has no Projects tab. */
  projectsTab?: ProjectsSeed;
  /** The pull requests, issues, tasks and commands the palette lists, and what it starts with pinned and opened. Mock only: Fleet serves the palette none of them yet. */
  palette?: import("../palette-fixtures").PaletteFixtures;
  /** Each repository's pull requests, by Manifest id, which the Projects tab's link search offers. */
  projectsPulls?: Readonly<Record<string, readonly import("../projects-tab/model").LinkedPull[]>>;
  /** The pages of pull requests a Session's ledger holds, as `readReviewPage` serves them and `writeReview` moves them, built per window. Mock only. */
  pullRequestPages?: () => PullRequestPages;
  /** What the one side panel reads a pull request, an issue, a bay and a Check from, built per window. Mock only: no Fleet serves these reads yet. */
  sharedPanel?: () => PanelReads;
  /** The tests that reach a pull request's changed files, for the Files tab's marks: the test map's read, which no Fleet serves to Bridge yet. Mock only. */
  testsReaching?: PanelReads["testsReaching"];
  /**
   * The shared home this machine is signed in to, and how each row stands against it. `staysHere` names the
   * rows only on this machine that a Sync cannot add, each with the sentence Fleet tells for it. Mock only.
   */
  sharedHome?: {
    account: string;
    repository: string;
    /** Every row as if every kind synced; `syncs` narrows what is drawn. */
    rows: readonly SharedRow[];
    staysHere?: readonly { kind: string; name: string; why: string }[];
    /** What syncs, switched from Kit. Absent follows the Account page's What syncs and draws no switches. */
    syncs?: Syncs;
  };
  /**
   * The machine starts signed out. Signing in as `signIn.login` shows `signIn.code` and sets up the
   * shared home `repository`; `add` is the second login a profile adds, with the items each of its
   * kinds would sync. Mock only.
   */
  account?: {
    signIn: { login: string; code: string; repository: string };
    add?: { login: string; code: string; items: Partial<Record<SyncKindWord, readonly SyncItem[]>> };
    /**
     * What each attempt at signing in comes to, in turn: ended this way once its code is typed, or refused
     * at once for these blank settings, or signs in (`"signs_in"`). Past the last, an attempt signs in.
     * Absent signs in at once.
     */
    attempts?: readonly ("signs_in" | { ends: EndedOutcome } | { blank: readonly BlankSetting[] })[];
    /** The machine starts signed in to these profiles, the first active, and shows the account already set up. */
    joined?: readonly { login: string; chosen: boolean }[];
    /** Logins whose sign-out leaves a row in the store, because the store is down. */
    storeDown?: readonly string[];
    /** Signing in makes no shared home, and What syncs is drawn this way until one exists. */
    noHome?: true;
    /** This machine's name, beside the page's title. */
    machine?: string;
    /** This machine's repositories as `owner/name`, each bound to the one profile whose login or organisation owns it. Absent draws none. */
    repositories?: readonly string[];
    /** The organisations each login belongs to, which matching reads beside the login. */
    orgs?: Readonly<Record<string, readonly string[]>>;
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

/** The state a connected Fleet publishes. */
export function connected(
  workflows: WorkflowSummary[],
  repositories: RepositorySummary[],
): BridgeState {
  return connectedOver(NOTHING_YET, workflows, repositories);
}

/** A connected Fleet holding exactly these repositories, for a test that needs a window no named scenario draws. */
export function onBoard(
  options: { workflows?: WorkflowSummary[]; repositories?: RepositorySummary[]; picked?: string | null } = {},
): Scenario {
  return onBoardOver<BridgeState, BridgeApi>(NOTHING_YET, options);
}
