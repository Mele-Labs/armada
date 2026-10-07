// What a mock scenario is made of, generic over the app's whole state `S` and API `A` so a surface
// can build scenarios without naming desktop's types. Desktop supplies the concrete ones
// (`mock/moment.ts`). `D` is the draft a scenario carries, which only the app that draws it names.

import { connectedTo, PROTOCOL_VERSION } from "@armada/protocol";
import type {
  Connection,
  Holdings,
  JobRetro,
  JobSummary,
  Lesson,
  ModelChoices,
  Outcome,
  RepositorySummary,
  Studio,
  WorkflowSummary,
  WorktreesHeld,
} from "@armada/protocol";
import { repository, workflow } from "@armada/screens/src/fixtures/build/base";
import type { JobFixture } from "@armada/screens/src/fixtures/fixture";
import { HARNESS } from "@armada/screens/src/fixtures/harness";

export { HARNESS };

/** The state fields `connected` and `onBoard` write; the app's whole state has all of them. */
export type PublishedCore = {
  connection: Connection;
  jobs: JobSummary[];
  readAt: number | null;
  holds: Holdings;
  repository: string | null;
};

/** One moment: what is published before anything is opened, and the reads behind each Job. */
export type Scenario<S, A, D = never> = {
  name: string;
  /** What it shows, as a sentence. The picker lists it. */
  says: string;
  state: S;
  /** Every Job's own reads, by Job id. A Job absent here opens onto `unanswered`. */
  reads: Record<string, JobFixture>;
  /** A Job to open on start, the way a pressed notification opens one. */
  opens?: string;
  /**
   * What this moment's boards draw that Fleet cannot serve yet.
   *
   * **Draft types, held on the scenario rather than on a read** — no operation
   * answers them, so there is no read to hold them on. A board built in this
   * milestone takes them as props; when a shape is promoted (#1545) it moves
   * onto the fixture's own reads and this field loses a key.
   */
  draft?: D;
  /**
   * The Studios this scenario's Fleet keeps. **Every scenario answers the Studio reads and
   * writes** (#1341), so one that names none keeps an empty list and the surface draws its empty
   * state rather than a read failure.
   */
  studios?: readonly Studio[];
  /**
   * Each Job's retro, by Job id, as `GET /jobs/:job_id/retro` answers. A Job the scenario holds
   * and this does not name is answered `pending`, which is what Fleet says of a Job not ended.
   */
  retros?: Record<string, JobRetro>;
  /** `GET /worktrees`. Absent is a read Fleet does not answer. */
  held?: WorktreesHeld;
  /** `GET /lessons`, newest retro first. Absent is a Fleet with no retro written. */
  lessons?: Lesson[];
  /**
   * Calls this scenario answers as Fleet would, over the fake's own. For a
   * flow whose answers depend on what was pressed before — Setup's edits and
   * Writes, a clone — which a fixed read cannot hold.
   */
  behaves?: (fleet: FleetHandle<S>) => Partial<A>;
  /**
   * What is published next, one entry each time a walk's `later` step moves on. A turn of the
   * merge line drawn moment by moment: nothing here arrives on its own clock, so a step is never
   * caught between two.
   */
  later?: readonly Partial<S>[];
};

/** What a scenario's `behaves` reaches: the state as published, and the one way to change it. */
export type FleetHandle<S> = {
  state: () => S;
  publish: (change: Partial<S>) => void;
};

/**
 * What a read the scenario holds nothing for comes back as. **A transport
 * failure, not a refusal**: a refusal carries a code, and nothing outside Fleet
 * may mint one (`refusedWith` in `@armada/protocol` says why).
 */
export function unanswered(path: string): Outcome {
  return {
    ok: false,
    why: "transport",
    detail: "Not in this mock scenario — no Fleet is behind this window",
    fault: { method: "GET", path, why: "unreachable" },
  };
}

/** A Fleet that answered. Invented: no process has this pid or this port. */
const CONNECTED: Connection = connectedTo(
  { protocolVersion: PROTOCOL_VERSION, pid: 4242, port: 7878, startedAt: "2026-09-10T14:00:00Z" },
  1,
);

/** What `list_models` answers — `props.ts`' own guess, so a story and the app agree. */
const MODELS: ModelChoices = { models: ["haiku", "sonnet", "opus"], default: "sonnet", harnesses: [HARNESS] };

/** The state a connected Fleet publishes over `nothingYet`, holding these Jobs. */
export function connected<S extends PublishedCore>(
  nothingYet: S,
  jobs: JobSummary[],
  workflows: WorkflowSummary[],
  repositories: RepositorySummary[],
): S {
  return {
    ...nothingYet,
    connection: CONNECTED,
    jobs,
    readAt: Date.now(),
    holds: {
      workflows,
      manifests: repositories.flatMap((one) => (one.manifest === undefined ? [] : [one.manifest])),
      models: MODELS,
      repositories,
    },
  };
}

/**
 * A connected Fleet holding exactly these rows, for a test that needs a Board
 * no named scenario draws — two repositories, one picked, a field changed.
 * `picked` is the rail's pick by root, as main holds it; absent is All.
 */
export function onBoard<S extends PublishedCore, A>(
  nothingYet: S,
  jobs: JobSummary[],
  {
    workflows = [workflow()],
    repositories = [repository()],
    picked = null,
  }: { workflows?: WorkflowSummary[]; repositories?: RepositorySummary[]; picked?: string | null } = {},
): Scenario<S, A> {
  return {
    name: "board",
    says: "A Board a test asked for",
    state: { ...connected(nothingYet, jobs, workflows, repositories), repository: picked },
    reads: {},
  };
}
