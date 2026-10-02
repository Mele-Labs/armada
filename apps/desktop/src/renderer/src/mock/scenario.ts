// A named moment for the whole app: the state main would publish, and every
// per-Job read behind it. Built from `@armada/screens`' fixtures, never beside
// them — `docs/practices/running-locally.md`, *Bridge on a mock Fleet*.

import type {
  JobSummary,
  ManifestSummary,
  RepositorySummary,
  WorkflowSummary,
} from "@armada/protocol";
import type { JobFixture } from "@armada/screens/src/fixtures/fixture";
import {
  running,
  workingAPlan,
  runningWaitingOnACommand,
  review,
  escalatedGateFailure,
  escalatedEvidenceSuspect,
  reviewAtDelivery,
  reviewAfterAnOverrule,
  reviewHeldByPolicy,
  queued,
  awaitingApproval,
  awaitingRepair,
  awaitingAttestation,
  piloted,
  escalatedBlockedByPolicy,
  escalatedInterrupted,
  escalatedSilent,
  escalatedLoopCap,
  escalatedNoReport,
  completedSuccess,
  completedFailed,
  rejected,
  killed,
  superseded,
  preparing,
  reading,
  unreadable,
  retryingCheckFailure,
  runningAtGate,
  gateChecksStreaming,
  proposing,
} from "@armada/screens/src/fixtures/build/index";
import { ARC_MOMENTS, dispatchTyping, everyTaskState } from "@armada/screens/src/fixtures/build/arc";
import type { ArcMoment } from "@armada/screens/src/fixtures/build/arc";
import { KIND_FIXTURES } from "@armada/screens/src/fixtures/build/kinds";
import { epicPlanReview, epicWave, membersMerged, membersStacked } from "@armada/screens/src/fixtures/build/waves";
import { agentText } from "@armada/screens/src/fixtures/build/markdown";
import { emptiedLine, mergeLines, neverLanded } from "@armada/screens/src/fixtures/build/merge-line";
import { everyDroneHad } from "@armada/screens/src/fixtures/build/drones-had";
import { repository, workflow } from "@armada/screens/src/fixtures/build/base";
import { recorded, RECORDED_SLUGS } from "@armada/screens/src/fixtures/recorded";
import realBoard from "@armada/screens/src/fixtures/boards/real-board.json";

import { NOTHING_YET } from "../../../shared/bridge";
import { heldByTheGamingCheck } from "./job-detail-fixtures";
import { connected } from "./moment";
import type { Scenario } from "./moment";
import { talking } from "./helm-fleet";
import { DRIFT_GONE, GH_ISSUE_VIEW, KIT_SERVERS, RUNS, manifesting } from "./manifest-fleet";
import { SCRATCH, SHEET_READ, settingUp } from "./setup-fleet";
import { EVERY_KIND_NAME, EVERY_KIND_STUDIO, everyKind, studying, untitled } from "./studio-fleet";
import { zoning } from "./studio-read-in";
import { readingNothing } from "./studio-read-nothing";
import { job2Landed } from "./job-2-landed";
import { job2AtReview } from "./job-2-at-review";
import { fillingIn } from "./proposer-fleet";

export { connected, onBoard, unanswered } from "./moment";
export type { FleetHandle, Scenario } from "./moment";

/** One of each, by manifest and id — two fixtures on one workflow list it once. */
function distinct<T>(items: T[], key: (item: T) => string): T[] {
  return [...new Map(items.map((item) => [key(item), item])).values()];
}

/**
 * The repository a Manifest is read from. **Invented where a fixture names only
 * the Manifest**: the root is made up, and nothing a screen draws reads it
 * except the rail's label, which is the Manifest's own `repository`.
 */
function servedFrom(manifest: ManifestSummary): RepositorySummary {
  return manifest.id === repository().manifest?.id
    ? repository()
    // Named for its repository, not its id: the recording's Manifest has the
    // id `armada`, which put it on the base repository's own folder and ticked
    // both in the picker.
    : { root: `/Users/user/${manifest.repository}`, records_root: manifest.records_root, manifest };
}

/**
 * A connected Fleet holding these fixtures' Jobs, each with its own reads.
 *
 * `alsoServed` is served beside what the Manifests imply. **Every repository a
 * fixture reaches is set up by construction** — `servedFrom` builds one per
 * Manifest — so a repository nobody set up can only arrive this way.
 */
function holding(
  name: string,
  says: string,
  fixtures: JobFixture[],
  { opens, alsoServed = [] }: { opens?: string; alsoServed?: RepositorySummary[] } = {},
): Scenario {
  const manifests = distinct(fixtures.flatMap((one) => one.manifests), (one) => one.id);
  return {
    name,
    says,
    state: connected(
      fixtures.map((one) => one.job),
      distinct(fixtures.flatMap((one) => one.workflows), (one) => `${one.manifest_id}/${one.id}`),
      [...manifests.map(servedFrom), ...alsoServed],
    ),
    reads: Object.fromEntries(fixtures.map((one) => [one.job.id, one])),
    opens,
  };
}

/** The same scenario with Fleet serving three lines: `armada land --status`, a quiet one, an empty one. */
function lined(scenario: Scenario): Scenario {
  const lines = [...mergeLines().lines, emptiedLine(NOTES.root), neverLanded(SCRATCH.root)];
  return { ...scenario, state: { ...scenario.state, mergeLines: { lines } } };
}

/**
 * One Job, already open, the way a pressed notification opens it — for a test
 * that moved a fixture to a moment no builder names. `whereOpen` is the
 * person's own preference for the Where things are section.
 */
export function onJob(fixture: JobFixture, { whereOpen = false }: { whereOpen?: boolean } = {}): Scenario {
  const scenario = holding("job", fixture.name, [fixture], { opens: fixture.job.id });
  return {
    ...scenario,
    state: { ...scenario.state, preferences: { ...scenario.state.preferences, where_things_are_open: whereOpen } },
  };
}

/**
 * The fixture, moved onto another id, handle and title.
 *
 * **Every `build/` fixture is the same Job** — one narrative at many
 * moments, `base.ts` says why — so on one Board they would be one row. Each
 * read that names its Job is moved with it, or the detail would draw a
 * different Job's reads as "not this one's".
 *
 * **The title is given, never taken from the builder's `name`.** That name is
 * the state the fixture demonstrates, and a row built from it read `running —
 * the drone is waiting for a person to allow a command` where a Job's title
 * reads `Cache the manifest read`. `EVERY_STATE_TITLES` is where the row's own
 * title is written.
 */
function asRow(fixture: JobFixture, at: number, slug: string, title: string): JobFixture {
  const id = `01M2C1TJ8G00${String(at).padStart(2, "0")}EVERYSTATE00`;
  const renamed = { id, handle: `${at}-${slug}`, title };
  const job = { ...fixture.job, ...renamed };
  const moved = <Read extends { state: string }>(read: Read): Read =>
    "jobId" in read ? { ...read, jobId: id } : read;
  const watched = moved(fixture.watched);
  return {
    ...fixture,
    job,
    watched:
      watched.state === "read"
        ? { ...watched, detail: { ...watched.detail, job: { ...watched.detail.job, ...renamed } } }
        : watched,
    observed: moved(fixture.observed),
    journalled: moved(fixture.journalled),
    resources: moved(fixture.resources),
    history: fixture.history === undefined ? undefined : moved(fixture.history),
    jobDrones: fixture.jobDrones === undefined ? undefined : moved(fixture.jobDrones),
    recorded: {
      footprint: moved(fixture.recorded.footprint),
      handed: moved(fixture.recorded.handed),
      evidence: moved(fixture.recorded.evidence),
      diff: moved(fixture.recorded.diff),
      remarks: moved(fixture.recorded.remarks),
    },
  };
}

/**
 * Every builder `fixtures/build/index.ts` exports, by its export name. **Named
 * one by one**, because the vocabulary gate refuses a wholesale import;
 * `scenario.test.ts` fails where this and `FIXTURES` disagree.
 */
export const BUILDERS = {
  running,
  workingAPlan,
  runningWaitingOnACommand,
  review,
  escalatedGateFailure,
  escalatedEvidenceSuspect,
  reviewAtDelivery,
  reviewAfterAnOverrule,
  reviewHeldByPolicy,
  queued,
  awaitingApproval,
  awaitingRepair,
  awaitingAttestation,
  piloted,
  escalatedBlockedByPolicy,
  escalatedInterrupted,
  escalatedSilent,
  escalatedLoopCap,
  escalatedNoReport,
  completedSuccess,
  completedFailed,
  rejected,
  killed,
  superseded,
  preparing,
  reading,
  unreadable,
  retryingCheckFailure,
  runningAtGate,
  gateChecksStreaming,
  proposing,
} satisfies Record<string, () => JobFixture>;

/** One builder's export name. `satisfies` above is what keeps this a union. */
type Builder = keyof typeof BUILDERS;

/**
 * The Job title each builder's `every-state` row carries, one per builder.
 *
 * **A builder's `name` is not a title.** It names the scenario and the row's
 * handle, and it reads as the state being demonstrated — a status word, a gate
 * or a Check, joined by an em dash to what happened. Drawn as a title it said
 * the same thing the status badge beside it already says, and no row read like
 * work anybody asked for (owner note, 17 Sep 2026). So each row carries a
 * change somebody asked for instead, in the voice `fixtures/build/board.ts`
 * writes the Board's rows in.
 *
 * Typed against `BUILDERS`, so a builder added without a title is a compile
 * error rather than a row back in the old shape.
 */
const EVERY_STATE_TITLES: Record<Builder, string> = {
  running: "Cache the manifest read between dispatches",
  workingAPlan: "Extract the column order selector into its own module",
  runningWaitingOnACommand: "Reuse one HTTP client across every query",
  review: "Fold the two notification routes into one",
  escalatedGateFailure: "Shorten the reconnect backoff to two seconds",
  escalatedEvidenceSuspect: "Prune the evidence bundle before it is written",
  reviewAtDelivery: "Carry the branch name into the pull request body",
  reviewAfterAnOverrule: "Name the attempt a step's log file belongs to",
  reviewHeldByPolicy: "Say on the Record why a step held for review",
  queued: "Move the worktree prune off the startup path",
  awaitingApproval: "Widen the allowlist to cover read-only git commands",
  awaitingRepair: "Coalesce the journal writes into one flush",
  awaitingAttestation: "Give the runtime file a version field",
  piloted: "Make the diff pane remember its split",
  escalatedBlockedByPolicy: "Teach the dispatcher to read a repository alias",
  escalatedInterrupted: "Record the port Fleet claimed in its own log",
  escalatedSilent: "Stop the log pane scrolling on a background write",
  escalatedLoopCap: "Round the cost estimate to the nearest cent",
  escalatedNoReport: "Collapse repeated journal notes into one row",
  completedSuccess: "Debounce the Job Board's resize handler",
  completedFailed: "Drop the second clock from the elapsed figure",
  rejected: "Send the manifest digest with every dispatch",
  killed: "Read the workflow file once per dispatch",
  superseded: "Sort the Job Board by when a job last moved",
  preparing: "Name the drone in the resources panel",
  reading: "Give every sheet its own scroll position",
  unreadable: "Keep the Helm dock open across a restart",
  retryingCheckFailure: "Trim the brief to the files the step touched",
  runningAtGate: "Hold the composer's draft while a job is open",
  gateChecksStreaming: "Let the palette open on an empty Job Board",
  proposing: "Let a Job’s settings sheet remember which section was open",
};

const BUILT: [Builder, JobFixture][] = Object.entries(BUILDERS).map(([name, make]) => [
  name as Builder,
  make(),
]);

/** Every recording, by its directory. */
const RECORDED: [string, JobFixture][] = RECORDED_SLUGS.map((slug) => [slug, recorded(slug)]);

/** Every builder as an `every-state` row, each on its own id and title. Held, so a Studio can name one. */
const EVERY_STATE_ROWS: JobFixture[] = BUILT.map(([name, fixture], at) =>
  asRow(fixture, at + 1, name, EVERY_STATE_TITLES[name]),
).map((fixture, at) => (at === 0 ? offAStudio(fixture) : fixture));

/**
 * The first row, dispatched off `everyKind()`'s Issue draft — #1362. **The row
 * the Studio already names**, so the two halves agree: the Board says *From a
 * Studio, dispatched by you*, and the detail's Studio row opens that Studio on
 * the node the Job arrived as.
 */
function offAStudio(fixture: JobFixture): JobFixture {
  const job = { ...fixture.job, origin: "studio_dispatched" };
  if (fixture.watched.state !== "read") return { ...fixture, job };
  return {
    ...fixture,
    job,
    watched: {
      ...fixture.watched,
      detail: {
        ...fixture.watched.detail,
        job,
        from_studio: { studio_id: EVERY_KIND_STUDIO, name: EVERY_KIND_NAME, node_id: "every-job" },
      },
    },
  };
}

/** `dispatchTyping` as the picker spells it: `arc/dispatch-typing`. */
function kebab(name: string): string {
  return name.replace(/([a-z])([A-Z])/g, "$1-$2").toLowerCase();
}

/**
 * One moment of the arc, or of a landing order, as a scenario.
 *
 * **The draft half is carried on the scenario and not on a read.** Nothing
 * Fleet serves answers it, so a board in this milestone takes it as props —
 * `Scenario.draft` says why, and where it goes once a shape is promoted.
 */
function moment(prefix: string, one: ArcMoment): Scenario {
  const built = holding(`${prefix}/${kebab(one.name)}`, one.says, one.fixtures, {
    opens: one.opens,
  });
  // A moment that has already typed a request is a moment inside the composer,
  // and the composer asks which repository first on All — so the rail opens on
  // the one this work is for, the way a person who got that far already has.
  // Every other moment opens a Job, where the pick changes nothing.
  const picked =
    one.draft.prompt === undefined ? null : (built.state.holds.repositories?.[0]?.root ?? null);
  return {
    ...built,
    draft: one.draft,
    // A proposer call still out is `BridgeState.proposing`, which is on the
    // wire — so it is published as state rather than carried as a draft.
    state: {
      ...built.state,
      repository: picked,
      // The questions this moment's Jobs are holding, published the way main
      // gathers them — the dock draws them, and so does the wave.
      ...(one.questions === undefined ? {} : { questions: one.questions }),
      ...(one.proposing === undefined ? {} : { proposing: one.proposing }),
    },
  };
}

/**
 * A second folder added by path and never set up, so that `NOTHING_SET_UP`
 * holds more than one — the surfaces that ask for a repository ask because
 * there are several, and one of them alone would not say so.
 */
const NOTES: RepositorySummary = {
  root: "/Users/user/notes",
  records_root: "/Users/user/Library/Application Support/Armada/records/notes",
};

/**
 * Repositories served, and not one of them with a Manifest. **The moment every
 * surface that needs a Manifest has nothing to offer**: the rail is on All
 * repositories, the ask lists only what is set up, and nothing is. Studios is
 * where it was found.
 */
const NOTHING_SET_UP: Scenario = {
  name: "nothing-set-up",
  says: "Two repositories served, neither set up",
  state: connected([], [], [SCRATCH, NOTES]),
  reads: {},
};

/** A Fleet that is not running: no runtime file, so nothing was ever connected. */
const NOT_RUNNING: Scenario = {
  name: "fleet-not-running",
  says: "Fleet is not running — no runtime file",
  state: {
    ...NOTHING_YET,
    connection: {
      state: "not_running",
      absence: {
        why: "no_runtime_file",
        path: "/Users/user/Library/Application Support/Armada/fleet.json",
      },
    },
  },
  reads: {},
};

/** The Board Fleet served on 11 Sep 2026, recorded with `scripts/record-job.mjs --board`. */
function recordedBoard(): Scenario {
  const board = realBoard as unknown as {
    jobs: JobSummary[];
    workflows: WorkflowSummary[];
    manifests: ManifestSummary[];
  };
  const reads = Object.fromEntries(
    RECORDED.map(([, fixture]) => fixture).filter((one) => board.jobs.some((job) => job.id === one.job.id))
      .map((one) => [one.job.id, one]),
  );
  return {
    name: "recorded-board",
    says: "The Board as Fleet served it on 11 Sep 2026",
    state: connected(board.jobs, board.workflows, board.manifests.map(servedFrom)),
    reads,
  };
}

/** The Job `held/gaming-check` opens on, as Fleet serves it with the Drone still there. */
const HELD_BY_A_FLAG = heldByTheGamingCheck(["override_verdict", "redirect_drone", "redispatch_job"]);

/**
 * Every scenario, by name. **The first is where the mock opens.**
 *
 * A recording added under `fixtures/recorded/` is a scenario and an `every-state`
 * row with no edit here; a builder needs its line in `BUILDERS`.
 */
export const SCENARIOS: readonly Scenario[] = [
  {
    ...holding(
      "every-state",
      "One Job in every state, each openable",
      [
        ...EVERY_STATE_ROWS,
        // A recording keeps its own id, which no built row shares. The one
        // recorded today is the Board's Cleared tab, which no builder reaches.
        ...RECORDED.map(([, fixture]) => fixture),
      ],
      // The folder somebody added and never set up. **Without it nobody
      // browsing the mock ever sees the greyed-out half of a repository
      // picker**: every repository here comes from a Manifest, so New job's
      // ask and Studios' ask drew nothing under Not set up, and the one
      // scenario that shows an unset repository — `nothing-set-up` — has no
      // set-up one to show it beside. `scratch` rather than a new name: it is
      // the folder Setup is run against, and the same folder in every
      // scenario that needs one nobody set up.
      { alsoServed: [SCRATCH] },
    ),
    // Studios worth looking at, the way the rows are Jobs worth looking at: every node kind and
    // every edge kind on one, and a second nobody has named — #1341.
    studios: [everyKind(EVERY_STATE_ROWS[0]!.job.id), untitled()],
  },
  NOT_RUNNING,
  NOTHING_SET_UP,
  {
    name: "first-launch",
    says: "Fleet running and serving no repository",
    state: connected([], [], []),
    reads: {},
  },
  {
    name: "empty-store",
    says: "One repository, and no Job yet",
    state: connected([], [workflow()], [repository()]),
    reads: {},
  },
  recordedBoard(),
  lined(
    holding(
      "merge-line",
      "Three repositories with a line: six in one, nobody in another, nothing ever landed in the third",
      EVERY_STATE_ROWS.filter((one) => ["running", "review", "queued"].some((slug) => one.job.handle.endsWith(`-${slug}`))),
      { alsoServed: [NOTES, SCRATCH] },
    ),
  ),
  settingUp({ repositories: [repository(), SCRATCH], sheet: SHEET_READ }),
  manifesting({ alwaysAllowed: [GH_ISSUE_VIEW], drift: DRIFT_GONE, kitServers: KIT_SERVERS, runs: RUNS }),
  studying().scenario,
  zoning().scenario,
  readingNothing().scenario,
  talking(),
  // The arc: one Feature Job from an empty prompt to a merge, one scenario per
  // moment. **The roster is walked**, so a moment added to `ARC_MOMENTS` is a
  // scenario here without a second edit.
  ...ARC_MOMENTS.map((one) => moment("arc", one)),
  // The arc's request dispatched, on a Fleet whose proposer fills its Job in a field at a time.
  fillingIn(moment("arc", dispatchTyping())),
  // A task in each of the six states, for Plan's marks; not an arc moment, so not in `ARC_MOMENTS`.
  moment("plan", everyTaskState()),
  // Several Jobs landing in order, and a wave under one plan. No kind name:
  // the scenario says what it draws (#1530, 22 Sep).
  moment("members", membersStacked()),
  moment("members", membersMerged()),
  moment("epic", epicWave()),
  moment("epic", epicPlanReview()),
  // What agents write, in markdown, at every surface that draws it.
  moment("markdown", agentText()),
  // One Job per workflow kind, on one Board and then one at a time.
  holding("kinds", "One Job per workflow kind, each on the steps its own file declares", [
    ...KIND_FIXTURES,
  ]),
  ...KIND_FIXTURES.map((fixture) =>
    holding(`kind/${fixture.job.workflow_id.replace(/_/g, "-")}`, fixture.name, [fixture], {
      opens: fixture.job.id,
    }),
  ),
  ...BUILT.map(([name, fixture]) => holding(`job/${name}`, fixture.name, [fixture], { opens: fixture.job.id })),
  ...RECORDED.map(([slug, fixture]) =>
    holding(`recorded/${slug}`, fixture.name, [fixture], { opens: fixture.job.id }),
  ),
  // The owner's Job 2 as `GET /jobs/2` served it: four groups Bridge stood in
  // for, every task still `open`, and a 40-character commit.
  holding("real/job-2-landed", job2Landed().name, [job2Landed()], { opens: job2Landed().job.id }),
  // The same Job just before it landed, at its review gate: the record the gate draws (#1680).
  holding("real/job-2-at-review", job2AtReview().name, [job2AtReview()], { opens: job2AtReview().job.id }),
  // A running Job and every Drone it has had, as `list_job_drones` serves them:
  // one killed, two finished with their cost, and the one running now.
  holding("drones/every-drone-had", everyDroneHad().name, [everyDroneHad()], { opens: everyDroneHad().job.id }),
  // A Job the gaming check holds with its Drone still on the step: a weakened
  // assertion and three refused commands, answered under the lead (#1672).
  holding("held/gaming-check", HELD_BY_A_FLAG.name, [HELD_BY_A_FLAG], { opens: HELD_BY_A_FLAG.job.id }),
];

/** The scenario by name, or `undefined` for a name nothing here holds. */
export function scenarioNamed(name: string): Scenario | undefined {
  return SCENARIOS.find((one) => one.name === name);
}
