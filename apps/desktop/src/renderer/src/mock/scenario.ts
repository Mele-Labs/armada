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
  handedInATask,
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
} from "@armada/jobs/fixtures/build/index";
import { ARC_MOMENTS, dispatchTyping, everyTaskState, executingHeld } from "@armada/jobs/fixtures/build/arc";
import type { ArcMoment } from "@armada/jobs/fixtures/build/arc";
import { groupChecking } from "@armada/jobs/fixtures/build/arc-checking";
import { KIND_FIXTURES, prototypeKind } from "@armada/jobs/fixtures/build/kinds";
import { epicPlanReview, epicWave, membersMerged, membersStacked } from "@armada/jobs/fixtures/build/waves";
import { waveOffTheWire } from "@armada/jobs/fixtures/build/wave-off-the-wire";
import { agentText } from "@armada/jobs/fixtures/build/markdown";
import { emptiedLine, mergeLines, neverLanded } from "@armada/screens/src/fixtures/build/merge-line";
import { repository, workflow } from "@armada/screens/src/fixtures/build/base";
import { recorded, RECORDED_SLUGS } from "@armada/screens/src/fixtures/recorded";
import realBoard from "@armada/screens/src/fixtures/boards/real-board.json";

import { NOTHING_YET } from "../../../shared/bridge";
import { connected } from "./moment";
import type { Scenario } from "./moment";
import { SCRATCH } from "./setup-fake";
import {
  answeringTheHeldCommand,
  evidenceRead,
  fillingIn,
  originsAndPanel,
  proposalFromAnIssue,
  retroFixtures,
  walkedPrototype,
  writingLogs,
} from "@armada/jobs/fake";
import { EVERY_KIND_NAME, EVERY_KIND_STUDIO, everyKind, untitled } from "@armada/studios/fake";
import { gridHeld, slotsHeld } from "@armada/cleanup/fake";
import { failingTurn, writingTheFailedLogs } from "./merge-line-turn";
import { asRow, holding, servedFrom } from "./holding";
import { scenariosOf } from "./slices";

export { connected, onBoard, unanswered } from "./moment";
export type { FleetHandle, Scenario } from "./moment";

/**
 * Every file in `scenarios/` is a row, one export a file. **Sorted here by export name**, because
 * a glob lists files in whatever order the filesystem gives and nothing else says where a row sits;
 * the names carry a rank for the rows that were here first. A name two files both export would
 * overwrite one with the other, which `scenario.test.ts` refuses.
 */
const LISTED: Scenario[] = Object.entries(
  Object.assign({}, ...Object.values(import.meta.glob<Record<string, Scenario>>("./scenarios/*.ts", { eager: true }))) as Record<string, Scenario>,
)
  .sort(([a], [b]) => (a < b ? -1 : a > b ? 1 : 0))
  .map(([, scenario]) => scenario);

/** The same scenario with Fleet serving three lines: `armada land --status`, a quiet one, an empty one. */
function lined(scenario: Scenario): Scenario {
  const lines = [...mergeLines().lines, emptiedLine(NOTES.root), neverLanded(SCRATCH.root)];
  return { ...scenario, state: { ...scenario.state, mergeLines: { lines } } };
}

/**
 * Three merge lines mid-turn, a Check failing in each, moment by moment: `later` carries the
 * three moments after the first, which a walk's `later` step publishes. The Job whose branch is in
 * the batch has the Check's failure in its own log, and that log moves with the lines.
 */
function failingCheck(): Scenario {
  const rows = EVERY_STATE_ROWS.filter((one) => ["running", "review", "queued"].some((slug) => one.job.handle.endsWith(`-${slug}`)));
  const base = holding(
    "merge-line-failed-check",
    "Three merge lines mid-turn, each with a Check failed and the turn still going",
    rows,
    { alsoServed: [NOTES, SCRATCH, BRIDGE] },
  );
  const running = rows.find((one) => one.job.handle.endsWith("-running"))!;
  const { first, later } = failingTurn(
    { armada: repository().root, notes: NOTES.root, scratch: SCRATCH.root, bridge: BRIDGE.root },
    running.journalled,
  );
  return { ...base, state: { ...base.state, ...first }, later, behaves: writingTheFailedLogs };
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
 * Every builder `fixtures/build/index.ts` exports, by its export name. **Named
 * one by one**, because the vocabulary gate refuses a wholesale import;
 * `scenario.test.ts` fails where this and `FIXTURES` disagree.
 */
export const BUILDERS = {
  running,
  workingAPlan,
  handedInATask,
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
  handedInATask: "Keep the plan board's selection when a task is handed in",
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

/** A fourth repository, whose line has waiting branches: `merge-line-failed-check`. */
const BRIDGE: RepositorySummary = {
  root: "/Users/user/bridge",
  records_root: "/Users/user/Library/Application Support/Armada/records/bridge",
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
  failingCheck(),
  // Each surface's own Fleet, listed by its slice: `slices.ts`.
  ...scenariosOf("setup", "manifest", "studios", "workflows", "helm"),
  // The arc: one Feature Job from an empty prompt to a merge, one scenario per
  // moment. **The roster is walked**, so a moment added to `ARC_MOMENTS` is a
  // scenario here without a second edit.
  ...ARC_MOMENTS.map((one) => moment("arc", one)),
  // The arc's request dispatched, on a Fleet whose proposer fills its Job in a field at a time.
  fillingIn(moment("arc", dispatchTyping())),
  // The same, with a Job at its gate beside it whose criteria came from an issue and the request.
  originsAndPanel(
    fillingIn(moment("arc", { ...dispatchTyping(), fixtures: [...dispatchTyping().fixtures, proposalFromAnIssue()] })),
  ),
  // A task in each of the six states, for Plan's marks; not an arc moment, so not in `ARC_MOMENTS`.
  moment("plan", everyTaskState()),
  // Not an arc moment, so not in `ARC_MOMENTS` and not in the arc's order. T5's Drone held on a command it was not given; answering it from the Drone's own card clears it.
  { ...moment("held", { ...executingHeld(), name: "command" }), behaves: answeringTheHeldCommand },
  // A Prototype held at Build, its mock up for review and opened in Bridge's window on the Job.
  walkedPrototype(onJob(evidenceRead(prototypeKind()))),
  // A Check's log, from both strips that draw one: group three's boundary running its Checks, and
  // the merge line's turn, each writing a log as it runs. Not an arc moment, so named here.
  lined({
    ...moment("arc", groupChecking()),
    name: "check-logs",
    says: "Group three's Checks and the merge line's turn, each writing its log",
    behaves: writingLogs,
  }),
  // Several Jobs landing in order, and a wave under one plan. No kind name:
  // the scenario says what it draws (#1530, 22 Sep).
  moment("members", membersStacked()),
  moment("members", membersMerged()),
  moment("epic", epicWave()),
  moment("epic", epicPlanReview()),
  // The same wave with no draft: each pass's line and each Job's edges as Fleet serves them (23.14).
  moment("epic", waveOffTheWire()),
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
  // Every row in `scenarios/`, by export name. **A walk's row is added there,
  // never here** — `docs/practices/list-files.md`.
  ...LISTED,
  // Job 3's retro and Job 2's, and the Lessons page over both (23.12): on Overview, and on Job 3.
  retros("retro/lessons", "Two Jobs' retros written, on Overview"),
  retros("retro/job-3", "Job 3, its retro written", { opensJob3: true }),
  pooled(),
  gridded(),
];

/** Cleanup over the worktree pool, one slot in each state; the first is a running Job's. */
function pooled(): Scenario {
  const running = EVERY_STATE_ROWS.find((one) => one.job.handle.endsWith("-running"))!;
  const scenario = holding("cleanup/slots", "The worktree pool on Cleanup, a slot in each state", [running]);
  return { ...scenario, held: slotsHeld(running.job, Date.now()) };
}

/**
 * Cleanup's one grid: a finished Job's bay holding files and an unmerged branch,
 * a running Job's bay, a stranded one, a closed one, and two worktrees outside
 * the pool, each with the acts that fit it.
 */
function gridded(): Scenario {
  const row = (suffix: string) => EVERY_STATE_ROWS.find((one) => one.job.handle.endsWith(suffix))!;
  const jobs = {
    finished: row("-completedSuccess"),
    running: row("-running"),
    rejected: row("-rejected"),
    failed: row("-completedFailed"),
  };
  const scenario = holding("cleanup/grid", "Cleanup as one grid of tiles, each opening its own panel", Object.values(jobs));
  return {
    ...scenario,
    held: gridHeld(
      { finished: jobs.finished.job, running: jobs.running.job, rejected: jobs.rejected.job, failed: jobs.failed.job },
      Date.now(),
    ),
  };
}

/** Job 3 and Job 2 with their retros written, and the Lessons listing over both. */
function retros(name: string, says: string, { opensJob3 = false }: { opensJob3?: boolean } = {}): Scenario {
  const { fixtures, retros: written, lessons } = retroFixtures();
  const scenario = holding(name, says, fixtures, opensJob3 ? { opens: fixtures[0]?.job.id } : {});
  return { ...scenario, retros: written, lessons };
}

/** The scenario by name, or `undefined` for a name nothing here holds. */
export function scenarioNamed(name: string): Scenario | undefined {
  return SCENARIOS.find((one) => one.name === name);
}
