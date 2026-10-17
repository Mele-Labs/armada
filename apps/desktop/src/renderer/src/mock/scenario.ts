// A named moment for the whole app: the state main would publish, and every
// per-Job read behind it. Built from `@armada/screens`' fixtures, never beside
// them — `docs/practices/running-locally.md`, *Bridge on a mock Fleet*.

import type { RepositorySummary } from "@armada/protocol";
import type { JobFixture } from "@armada/screens/src/fixtures/fixture";
import {
  running,
  review,
  queued,
  completedSuccess,
  completedFailed,
  rejected,
} from "@armada/screens/src/fixtures/build/index";
import { emptiedLine, mergeLines, neverLanded } from "@armada/screens/src/fixtures/build/merge-line";
import { repository, workflow } from "@armada/screens/src/fixtures/build/base";
import { asOfNow } from "@armada/screens/src/fixtures/as-of-now";

import { NOTHING_YET } from "../../../shared/bridge";
import { connected } from "./moment";
import type { Scenario } from "./moment";
import { SCRATCH } from "./setup-fake";
import { retroFixtures } from "@armada/screens/src/fixtures/jobs/job-3-retro";
import { everyKind, untitled } from "@armada/studios/fake";
import { gridHeld, slotsHeld } from "@armada/cleanup/fake";
import { failingTurn, writingTheFailedLogs } from "./merge-line-turn";
import { asRow, holding } from "./holding";
import { scenariosOf } from "./slices";

export { connected, onBoard, unanswered } from "./moment";
export type { FleetHandle, Scenario } from "./moment";

/**
 * Every file in `scenarios/` is a row, one export a file. **Sorted here by export name**, because
 * a glob lists files in whatever order the filesystem gives and nothing else says where a row sits;
 * the names carry a rank for the rows that were here first. A name two files both export would
 * overwrite one with the other, which `scenario.test.ts` refuses.
 */
const ROW_FILES = (
  import.meta as ImportMeta & {
    glob(pattern: string, options: { eager: true }): Record<string, Record<string, Scenario>>;
  }
).glob("./scenarios/*.ts", { eager: true });

const LISTED: Scenario[] = Object.entries(Object.assign({}, ...Object.values(ROW_FILES)) as Record<string, Scenario>)
  .sort(([a], [b]) => (a < b ? -1 : a > b ? 1 : 0))
  .map(([, scenario]) => scenario);

/** The same scenario with Fleet serving three lines: one with a queue and outcomes, a quiet one, an empty one. */
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
  const rows = ROWS.filter((one) => ["running", "review", "queued"].some((slug) => one.job.handle.endsWith(`-${slug}`)));
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
 * The Jobs Fleet still serves behind the scenarios that read them — a merge line's batch, a
 * worktree's holder — each on its own id and title, moved onto the real clock (#1505).
 */
const BUILDERS = { running, review, queued, completedSuccess, completedFailed, rejected };

const TITLES: Record<keyof typeof BUILDERS, string> = {
  running: "Cache the manifest read between dispatches",
  review: "Fold the two notification routes into one",
  queued: "Move the worktree prune off the startup path",
  completedSuccess: "Debounce the resize handler",
  completedFailed: "Drop the second clock from the elapsed figure",
  rejected: "Send the manifest digest with every dispatch",
};

const ROWS: JobFixture[] = (Object.keys(BUILDERS) as (keyof typeof BUILDERS)[]).map((name, at) =>
  asRow(asOfNow(BUILDERS[name]()), at + 1, name, TITLES[name]),
);

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
 * A person working across many repositories: the picker's list is long enough that finding
 * one is the cost, so it grows a filter field and leads with the last few picked.
 */
const MANY: RepositorySummary[] = [
  "billing-api", "billing-web", "checkout", "docs-site", "infra", "ios-app", "android-app",
  "data-pipeline", "design-system", "auth-service", "search", "notifications", "analytics",
  "admin-console", "marketing-site", "mobile-sdk", "payments", "reporting", "scheduler", "armada-fork",
].map((name) => ({
  root: `/Users/user/code/${name}`,
  records_root: `/Users/user/Library/Application Support/Armada/records/${name}`,
}));

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

/**
 * Every scenario, by name. **The first is where the mock opens.**
 */
export const SCENARIOS: readonly Scenario[] = [
  {
    ...holding(
      "default",
      "Fleet serving two repositories, a Studio worth looking at, and a Session in none",
      [ROWS[0]!],
      // The folder somebody added and never set up. **Without it nobody
      // browsing the mock ever sees the greyed-out half of a repository
      // picker**: every repository here comes from a Manifest, so the ask
      // drew nothing under Not set up, and the one scenario that shows an
      // unset repository — `nothing-set-up` — has no set-up one to show it
      // beside. `scratch` is the folder Setup is run against.
      { alsoServed: [SCRATCH] },
    ),
    // Studios worth looking at: every node kind and every edge kind on one, and a second nobody
    // has named — #1341.
    studios: [everyKind(ROWS[0]!.job.id), untitled()],
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
    says: "One repository, and no Session yet",
    state: connected([], [workflow()], [repository()]),
    reads: {},
  },
  lined(
    holding(
      "merge-line",
      "Three repositories with a line: six in one, nobody in another, nothing ever landed in the third",
      ROWS.filter((one) => ["running", "review", "queued"].some((slug) => one.job.handle.endsWith(`-${slug}`))),
      { alsoServed: [NOTES, SCRATCH] },
    ),
  ),
  failingCheck(),
  holding(
    "many-repositories",
    "Twenty-one repositories served, so the picker has a filter field and leads with the last few picked",
    ROWS.filter((one) => ["running", "review"].some((slug) => one.job.handle.endsWith(`-${slug}`))),
    { alsoServed: MANY },
  ),
  // Each surface's own Fleet, listed by its slice: `slices.ts`.
  ...scenariosOf("setup", "manifest", "studios", "workflows", "helm"),
  // Every row in `scenarios/`, by export name. **A walk's row is added there,
  // never here** — `docs/practices/list-files.md`.
  ...LISTED,
  // Job 3's retro and Job 2's, and the Lessons page over both (23.12).
  retros("retro/lessons", "Two retros written, on Overview"),
  pooled(),
  gridded(),
];

/** Cleanup over the worktree pool, one slot in each state; the first is a running Job's. */
function pooled(): Scenario {
  const running = ROWS.find((one) => one.job.handle.endsWith("-running"))!;
  const scenario = holding("cleanup/slots", "The worktree pool on Cleanup, a slot in each state", [running]);
  return { ...scenario, held: slotsHeld(running.job, Date.now()) };
}

/**
 * Cleanup's one grid: a finished Job's bay holding files and an unmerged branch,
 * a running Job's bay, a stranded one, a closed one, and two worktrees outside
 * the pool, each with the acts that fit it.
 */
function gridded(): Scenario {
  const row = (suffix: string) => ROWS.find((one) => one.job.handle.endsWith(suffix))!;
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

/** Two Jobs with their retros written, and the Lessons listing over both. */
function retros(name: string, says: string): Scenario {
  const { fixtures, retros: written, lessons } = retroFixtures();
  const scenario = holding(name, says, fixtures);
  return { ...scenario, retros: written, lessons };
}

/** The scenario by name, or `undefined` for a name nothing here holds. */
export function scenarioNamed(name: string): Scenario | undefined {
  return SCENARIOS.find((one) => one.name === name);
}