// A named moment for the whole app: the state main would publish. Built from
// `@armada/screens`' fixtures, never beside them — `docs/practices/running-locally.md`, *Bridge on a mock Fleet*.

import type { RepositorySummary } from "@armada/protocol";
import { emptiedLine, mergeLines, neverLanded } from "@armada/screens/src/fixtures/merge-line";
import { repository, workflow } from "@armada/screens/src/fixtures/repository";

import { NOTHING_YET } from "../../../shared/bridge";
import { connected } from "./moment";
import type { Scenario } from "./moment";
import { SCRATCH } from "./setup-fake";
import { retroFixtures } from "@armada/screens/src/fixtures/retros";
import { everyKind, untitled } from "@armada/studios/fake";
import { gridHeld, slotsHeld } from "@armada/cleanup/fake";
import { failingTurn, writingTheFailedLogs } from "./merge-line-turn";
import { holding } from "./holding";
import { sessionsStore } from "./sessions/script";
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
  // A file may also export plain values (`job-notes.ts` does): only an object is a scenario row.
  .filter(([, scenario]) => typeof scenario === "object")
  .sort(([a], [b]) => (a < b ? -1 : a > b ? 1 : 0))
  .map(([, scenario]) => scenario);

/** The same scenario with Fleet serving three lines: one with a queue and outcomes, a quiet one, an empty one. */
function lined(scenario: Scenario): Scenario {
  const lines = [...mergeLines().lines, emptiedLine(NOTES.root), neverLanded(SCRATCH.root)];
  return { ...scenario, state: { ...scenario.state, mergeLines: { lines } } };
}


/**
 * Three merge lines mid-turn, a Check failing in each, moment by moment: `later` carries the
 * three moments after the first, which a walk's `later` step publishes.
 */
function failingCheck(): Scenario {
  const base = holding(
    "merge-line-failed-check",
    "Three merge lines mid-turn, each with a Check failed and the turn still going",
    [repository(), NOTES, SCRATCH, BRIDGE],
  );
  const { first, later } = failingTurn({ armada: repository().root, notes: NOTES.root, scratch: SCRATCH.root, bridge: BRIDGE.root });
  return { ...base, state: { ...base.state, ...first }, later, behaves: writingTheFailedLogs };
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
  // Fleet gives each the id a Job started there carries.
  state: connected([], [{ ...SCRATCH, implicit_id: "scratch-4f2a9c" }, { ...NOTES, implicit_id: "notes-81d3be" }]),
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
};

/**
 * Every scenario, by name. **The first is where the mock opens.**
 */
export const SCENARIOS: readonly Scenario[] = [
  {
    ...holding(
      "default",
      "Fleet serving two repositories, a Studio worth looking at, and a Session in none",
      // The folder somebody added and never set up. **Without it nobody
      // browsing the mock ever sees the greyed-out half of a repository
      // picker**: every repository here comes from a Manifest, so the ask
      // drew nothing under Not set up, and the one scenario that shows an
      // unset repository — `nothing-set-up` — has no set-up one to show it
      // beside. `scratch` is the folder Setup is run against.
      [repository(), SCRATCH],
    ),
    // Studios worth looking at: every node kind and every edge kind on one, and a second nobody
    // has named — #1341.
    studios: [everyKind(), untitled()],
  },
  NOT_RUNNING,
  NOTHING_SET_UP,
  {
    name: "first-launch",
    says: "Fleet running and serving no repository",
    state: connected([], []),
  },
  {
    name: "empty-store",
    says: "One repository, and no Session yet",
    state: connected([workflow()], [repository()]),
  },
  lined(
    holding(
      "merge-line",
      "Three repositories with a line: six in one, nobody in another, nothing ever landed in the third",
      [repository(), NOTES, SCRATCH],
    ),
  ),
  failingCheck(),
  manyRepositories(),
  // Each surface's own Fleet, listed by its slice: `slices.ts`.
  ...scenariosOf("setup", "manifest", "studios", "workflows", "helm"),
  // Every row in `scenarios/`, by export name. **A walk's row is added there,
  // never here** — `docs/practices/list-files.md`.
  ...LISTED,
  // Two Sessions' retros, and the Lessons page over both (23.12).
  retros("retro/lessons", "Two retros written, on Overview"),
  pooled(),
  gridded(),
];

/** Cleanup over the worktree pool, one slot in each state. */
function pooled(): Scenario {
  const scenario = holding("cleanup/slots", "The worktree pool on Cleanup, a slot in each state");
  return { ...scenario, held: slotsHeld(Date.now()) };
}

/**
 * Cleanup's one grid: bays held by Sessions, a stranded one, a closed one, and two worktrees
 * outside the pool, each with the acts that fit it.
 */
function gridded(): Scenario {
  const scenario = holding("cleanup/grid", "Cleanup as one grid of tiles, each opening its own panel");
  return { ...scenario, held: gridHeld(Date.now()) };
}

/** Two Sessions with their retros written, and the Lessons listing over both. */
function retros(name: string, says: string): Scenario {
  const { retros: written, lessons } = retroFixtures();
  return { ...holding(name, says), retros: written, lessons };
}

/** The scenario by name, or `undefined` for a name nothing here holds. */
export function scenarioNamed(name: string): Scenario | undefined {
  return SCENARIOS.find((one) => one.name === name);
}
/** Twenty-one repositories served and Sessions served, so a Job started in one makes it a recent pick. */
function manyRepositories(): Scenario {
  const served = [repository(), ...MANY];
  const base = holding("many-repositories", "Twenty-one repositories served, so the picker has a filter field and leads with the last few picked", served);
  const owners = new Map(served.flatMap((one) => (one.manifest === undefined ? [] : [[one.root, one.manifest.id] as const])));
  return { ...base, draft: { ...base.draft, sessions: () => sessionsStore([], [], owners) } };
}
