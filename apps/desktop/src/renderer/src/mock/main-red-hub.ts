// Main going red on CI, three times, one moment at a time: what the owner sees and does. The hub's
// head, its pull requests and the Job on the red are what Fleet serves as the line's `hub`, and a
// Job's part in the red rides on its row as `fixes_main` (23.41, 23.42). The Jobs the work can go
// back to are read off the Board's rows. `main-goes-red.ts` walks it.
//
//   1  The Job's own pull request merges and main goes red. That Job was watching its landing, so it
//      takes the red itself and nobody is asked.
//   2  A person merges a Job's pull request by hand, so nothing was watching it. The owner is asked,
//      and sends the work back to that Job, listed first among the recent ones.
//   3  A person's own pull request turns it red. The owner is asked and dispatches a new Job.
//
// Each ends with the fix landing: main green, and the Job that fixed it says so on its row.

import type { HubJob, HubPull, HubSession, MainRed, MainState, RecentSession } from "@armada/components";
import type { FollowedLandLog, HubMerged, HubPullRequest, JobSummary, LandCheckAt, MainStanding, MergeLine, MergeLineHub, MergeLines } from "@armada/protocol";
import type { FixesMain } from "@armada/screens/src/main-red";
import type { JobFixture } from "@armada/screens/src/fixtures/fixture";
import { completedSuccess, queued, review, running } from "@armada/jobs/fixtures/build/index";

import type { BridgeApi } from "../../../shared/api";
import type { BridgeState } from "../../../shared/bridge";
import { asRow } from "./holding";
import type { FleetHandle } from "./moment";

const PULL = "https://git.example/armada/pull/";

/** The Jobs on the Board, by the number each is drawn under. */
export const CACHE = asRow(running(), 60, "cache", "Cache the manifest read between dispatches");
export const DEBOUNCE = asRow(running(), 61, "debounce", "Debounce the Job Board's resize handler");
const NOTIFY = asRow(completedSuccess(), 62, "notify", "Fold the two notification routes into one");
const ZONE = asRow(review(), 63, "zone", "Name the zone a read-in lands in");
const PAUSE = asRow(queued(), 64, "pause", "Store a pause marker on the Job");
/** Dispatched by the owner in the third incident, so it is on no Board until then. */
export const FIX = asRow(running(), 65, "fix-main", "Fix components_test on main");

/** Every Job the story holds, for the scenario's reads. */
export const FIXTURES: JobFixture[] = [CACHE, DEBOUNCE, NOTIFY, ZONE, PAUSE, FIX];

const branchOf = (one: JobFixture) => `armada/${one.job.handle}`;
const hubJob = (one: JobFixture): HubJob => ({ id: one.job.id, title: one.job.title });

/** A Job's row at a moment: its status, and its part in main's red where it has one. */
function rowOf(one: JobFixture, status: string, fixes?: FixesMain): JobSummary {
  const job = { ...one.job, status, branch: branchOf(one), ...(fixes === undefined ? {} : { fixes_main: fixes }) };
  return job as JobSummary;
}

/** The three reds: what failed, its test, and the pull request that merged it. */
const FIRST: MainRed = {
  check: "screens_test",
  test: "manifest-read.test.ts > reads the file again once it changes",
  merge: { number: 1812, url: `${PULL}1812`, branch: branchOf(CACHE), job: hubJob(CACHE) },
};
const SECOND: MainRed = {
  check: "desktop_test",
  test: "resources-poll.test.ts > a reading that stops polls nothing",
  merge: { number: 1816, url: `${PULL}1816`, branch: branchOf(DEBOUNCE), job: hubJob(DEBOUNCE) },
};
const THIRD: MainRed = {
  check: "components_test",
  test: "Theme tokens > the dark ground keeps its contrast",
  merge: { number: 1815, url: `${PULL}1815`, branch: "nick/theme-tokens" },
};

/** A CI job that maps to no Manifest Check, and a log with no test Armada can read out of it. */
const FOURTH: MainRed = {
  check: "test-all",
  unmapped: true,
  merge: { number: 1814, url: `${PULL}1814`, branch: "studio/zone-proposal", job: hubJob(ZONE) },
};

const fixing = (red: MainRed): FixesMain => ({
  state: "fixing",
  check: red.check,
  ...(red.test === undefined ? {} : { test: red.test }),
  merge: red.merge!.number,
});
const fixed = (red: MainRed, in_: number): FixesMain => ({ ...fixing(red), state: "fixed", fixed_in: in_ });

const pull = (number: number, branch: string, ci: HubPull["ci"], job?: JobFixture): HubPull => ({
  number,
  url: `${PULL}${number}`,
  branch,
  ci,
  ...(job === undefined ? {} : { job: hubJob(job) }),
});

type Moment = {
  main: MainState;
  /** The pull requests open now, by how each one's `ci` stands. */
  pulls: HubPull[];
  jobs: JobSummary[];
};

/** The Jobs that are not part of the story, and the pull request of each. */
const BYSTANDING = (zone: HubPull["ci"], pause: HubPull["ci"]): { pulls: HubPull[]; jobs: JobSummary[] } => ({
  pulls: [pull(1814, "studio/zone-proposal", zone, ZONE), pull(1819, "fleet/pause-markers", pause, PAUSE)],
  jobs: [rowOf(NOTIFY, "completed_success"), rowOf(ZONE, "awaiting_review"), rowOf(PAUSE, "queued")],
});

const PERSON = pull(1815, "nick/theme-tokens", "passed");

/** Each moment of the story. The first is what a window opens on. */
const MOMENTS: Moment[] = [
  // 0  Green, with the Job's pull request about to merge.
  {
    main: { state: "green" },
    pulls: [
      pull(1812, branchOf(CACHE), "passed", CACHE),
      pull(1816, branchOf(DEBOUNCE), "passed", DEBOUNCE),
      PERSON,
      ...BYSTANDING("passed", "running").pulls,
    ],
    jobs: [rowOf(CACHE, "awaiting_review"), rowOf(DEBOUNCE, "awaiting_review"), ...BYSTANDING("passed", "running").jobs],
  },
  // 1  #1812 merged and main is red. The Job that merged it was watching, so it has the red already.
  {
    main: { state: "red", red: FIRST, taken: hubJob(CACHE) },
    pulls: [
      pull(1816, branchOf(DEBOUNCE), "waiting_on_main", DEBOUNCE),
      { ...PERSON, ci: "waiting_on_main" },
      ...BYSTANDING("waiting_on_main", "failed").pulls,
    ],
    jobs: [rowOf(CACHE, "running", fixing(FIRST)), rowOf(DEBOUNCE, "awaiting_review"), ...BYSTANDING("waiting_on_main", "failed").jobs],
  },
  // 2  Its fix landed in #1821: green, and the Job says it fixed it.
  {
    main: { state: "green" },
    pulls: [pull(1816, branchOf(DEBOUNCE), "passed", DEBOUNCE), PERSON, ...BYSTANDING("passed", "failed").pulls],
    jobs: [rowOf(CACHE, "completed_success", fixed(FIRST, 1821)), rowOf(DEBOUNCE, "awaiting_review"), ...BYSTANDING("passed", "failed").jobs],
  },
  // 3  A person merged the Job's pull request on the forge, so nothing was watching it.
  {
    main: { state: "red", red: SECOND },
    pulls: [{ ...PERSON, ci: "waiting_on_main" }, ...BYSTANDING("waiting_on_main", "failed").pulls],
    jobs: [rowOf(CACHE, "completed_success", fixed(FIRST, 1821)), rowOf(DEBOUNCE, "completed_success"), ...BYSTANDING("waiting_on_main", "failed").jobs],
  },
  // 4  The owner sent the work back to it, and it is running.
  {
    main: { state: "red", red: SECOND, taken: hubJob(DEBOUNCE) },
    pulls: [{ ...PERSON, ci: "waiting_on_main" }, ...BYSTANDING("waiting_on_main", "failed").pulls],
    jobs: [rowOf(CACHE, "completed_success", fixed(FIRST, 1821)), rowOf(DEBOUNCE, "running", fixing(SECOND)), ...BYSTANDING("waiting_on_main", "failed").jobs],
  },
  // 5  #1822 landed.
  {
    main: { state: "green" },
    pulls: [PERSON, ...BYSTANDING("passed", "failed").pulls],
    jobs: [rowOf(CACHE, "completed_success", fixed(FIRST, 1821)), rowOf(DEBOUNCE, "completed_success", fixed(SECOND, 1822)), ...BYSTANDING("passed", "failed").jobs],
  },
  // 6  A person's own pull request turned it red.
  {
    main: { state: "red", red: THIRD },
    pulls: BYSTANDING("waiting_on_main", "failed").pulls,
    jobs: [rowOf(CACHE, "completed_success", fixed(FIRST, 1821)), rowOf(DEBOUNCE, "completed_success", fixed(SECOND, 1822)), ...BYSTANDING("waiting_on_main", "failed").jobs],
  },
  // 7  The owner dispatched a new Job, and it is running.
  {
    main: { state: "red", red: THIRD, taken: hubJob(FIX) },
    pulls: BYSTANDING("waiting_on_main", "failed").pulls,
    jobs: [
      rowOf(CACHE, "completed_success", fixed(FIRST, 1821)),
      rowOf(DEBOUNCE, "completed_success", fixed(SECOND, 1822)),
      rowOf(FIX, "running", fixing(THIRD)),
      ...BYSTANDING("waiting_on_main", "failed").jobs,
    ],
  },
  // 8  #1823 landed.
  {
    main: { state: "green" },
    pulls: BYSTANDING("passed", "failed").pulls,
    jobs: [
      rowOf(CACHE, "completed_success", fixed(FIRST, 1821)),
      rowOf(DEBOUNCE, "completed_success", fixed(SECOND, 1822)),
      rowOf(FIX, "completed_success", fixed(THIRD, 1823)),
      ...BYSTANDING("passed", "failed").jobs,
    ],
  },
  // 9  The Job behind #1814 merged it and was watching. Its CI job maps to no Check, and no test could be read.
  {
    main: { state: "red", red: FOURTH, taken: hubJob(ZONE) },
    pulls: [pull(1819, "fleet/pause-markers", "failed", PAUSE)],
    jobs: [
      rowOf(CACHE, "completed_success", fixed(FIRST, 1821)),
      rowOf(DEBOUNCE, "completed_success", fixed(SECOND, 1822)),
      rowOf(FIX, "completed_success", fixed(THIRD, 1823)),
      rowOf(ZONE, "running", fixing(FOURTH)),
      rowOf(NOTIFY, "completed_success"),
      rowOf(PAUSE, "queued"),
    ],
  },
  // 10  #1824 landed.
  {
    main: { state: "green" },
    pulls: [pull(1819, "fleet/pause-markers", "failed", PAUSE)],
    jobs: [
      rowOf(CACHE, "completed_success", fixed(FIRST, 1821)),
      rowOf(DEBOUNCE, "completed_success", fixed(SECOND, 1822)),
      rowOf(FIX, "completed_success", fixed(THIRD, 1823)),
      rowOf(ZONE, "completed_success", fixed(FOURTH, 1824)),
      rowOf(NOTIFY, "completed_success"),
      rowOf(PAUSE, "queued"),
    ],
  },
];

const COMMIT = "9f3c1d2e4b5a69788796a5b4c3d2e1f001122334";
const READ_AT = "2026-10-06T10:00:00Z";

/** Main as Fleet serves it: the failing job named as the forge names it, and a Check only where it maps. */
function standing(main: MainState): MainStanding {
  if (main.state === "green") return { state: "green", commit: COMMIT, read_at: READ_AT };
  const { red } = main;
  const { merge } = red;
  return {
    state: "red",
    commit: COMMIT,
    read_at: READ_AT,
    red_since: READ_AT,
    failed: [
      {
        name: red.check,
        ...(red.unmapped === true ? {} : { check: red.check }),
        ...(red.test === undefined ? {} : { tests: [red.test] }),
      },
    ],
    ...(merge === undefined
      ? {}
      : {
          merge: {
            number: merge.number,
            url: merge.url,
            branch: merge.branch,
            ...(merge.job === undefined ? {} : { job: merge.job }),
            ...(merge.session === undefined ? {} : { session: merge.session }),
          },
        }),
  };
}

const requestOf = (one: HubPull): HubPullRequest => ({
  number: one.number,
  title: one.branch,
  branch: one.branch,
  url: one.url,
  ci: one.ci,
  ...(one.job === undefined ? {} : { job: one.job }),
});

/**
 * A pull request that merged, as the forge lists it, with the CI run on its merge commit. `at` is the
 * moment it lands; a list is every one that has, newest first, up to what the panel keeps.
 */
type Landing = { at: number; number: number; branch: string; owner?: { job: HubJob } | { session: HubSession }; run: "passed" | "failed"; failed?: string };

const KEPT = 5;
const hex = (n: number) => n.toString(16).padStart(40, "0");

function mergedAt(landings: readonly Landing[], n: number): HubMerged[] {
  return landings
    .filter((one) => one.at <= n)
    .sort((a, b) => b.at - a.at || b.number - a.number)
    .slice(0, KEPT)
    .map((one) => ({
      number: one.number,
      title: one.branch,
      branch: one.branch,
      url: `${PULL}${one.number}`,
      merged_at: READ_AT,
      commit: hex(one.number),
      ...(one.owner === undefined ? {} : one.owner),
      main_run: { state: one.run, ...(one.failed === undefined ? {} : { failed: [one.failed] }) },
    }));
}

/** The line a moment serves for the repository, with the hub on it. */
function lineAt(root: string, at: number, moments: readonly Moment[] = MOMENTS, sessions: readonly RecentSession[] = [], landings: readonly Landing[] = LANDINGS): MergeLines {
  const { main, pulls } = moments[at]!;
  const hub: MergeLineHub = {
    main: standing(main),
    pull_requests: pulls.map(requestOf),
    ...(main.state === "red" && main.taken !== undefined ? { fixing: main.taken } : {}),
    ...(sessions.length === 0 ? {} : { recent_sessions: [...sessions] }),
    merged: mergedAt(landings, at),
  };
  return { lines: [{ root, line: [], off: [], landed: [], sent_back: [], hub } as MergeLine] };
}

/** What merged before the story and during it, in the order the moments land it. */
const LANDINGS: Landing[] = [
  { at: 0, number: 1799, branch: "nick/ports-cleanup", run: "passed" },
  { at: 0, number: 1803, branch: branchOf(NOTIFY), owner: { job: hubJob(NOTIFY) }, run: "passed" },
  { at: 0, number: 1808, branch: "fleet/pause-markers-read", run: "passed" },
  { at: 1, number: 1812, branch: branchOf(CACHE), owner: { job: hubJob(CACHE) }, run: "failed", failed: "screens_test" },
  { at: 2, number: 1821, branch: `${branchOf(CACHE)}-fix`, owner: { job: hubJob(CACHE) }, run: "passed" },
  { at: 3, number: 1816, branch: branchOf(DEBOUNCE), owner: { job: hubJob(DEBOUNCE) }, run: "failed", failed: "desktop_test" },
  { at: 5, number: 1822, branch: `${branchOf(DEBOUNCE)}-fix`, owner: { job: hubJob(DEBOUNCE) }, run: "passed" },
  { at: 6, number: 1815, branch: "nick/theme-tokens", run: "failed", failed: "components_test" },
  { at: 8, number: 1823, branch: branchOf(FIX), owner: { job: hubJob(FIX) }, run: "passed" },
  { at: 9, number: 1814, branch: "studio/zone-proposal", owner: { job: hubJob(ZONE) }, run: "failed", failed: "test-all" },
  { at: 10, number: 1824, branch: `${branchOf(ZONE)}-fix`, owner: { job: hubJob(ZONE) }, run: "passed" },
];

/** The state at the first moment, and each moment after as the change a walk publishes. */
export function mainRed(root: string): { first: Partial<BridgeState>; later: Partial<BridgeState>[] } {
  const at = (n: number): Partial<BridgeState> => ({ mergeLines: lineAt(root, n), jobs: MOMENTS[n]!.jobs });
  return { first: at(0), later: MOMENTS.slice(1).map((_, n) => at(n + 1)) };
}

// A Session's pull request turned main red. Fleet names the Session that owns the pull request, and
// the send-back dialog lists Sessions beside Jobs. `main-red-session` walks it.
//
//   0  The Session's pull request is open and green.
//   1  It merged and main is red: the head names the Session, and the owner is asked.
//   2  The owner sent the work back to the Session, and its fix landed.

const SESSION: HubSession = { id: "session-c279e883", title: "Show who merged on the head" };
const SESSION_BRANCH = "armada/session-c279e883";

/** Newest first, the culprit not first, so the dialog has to put it there. */
const RECENT_SESSIONS: RecentSession[] = [
  { id: "session-3d90b1a2", title: "Rework the Cockpit deck cards", branch: "armada/session-3d90b1a2" },
  { ...SESSION, branch: SESSION_BRANCH },
];

const SESSION_RED: MainRed = {
  check: "screens_test",
  test: "merge-line.test.ts > folds a Session's pull request onto the head",
  merge: { number: 1830, url: `${PULL}1830`, branch: SESSION_BRANCH, session: SESSION },
};

/** A pull request whose branch and owner's title are both longer than its row: both are cut short. */
const LONG_JOB: HubJob = { id: "job-wire-the-cockpit", title: "Wire the Cockpit's Job detail panel and Job Board" };
const longPull = (ci: HubPull["ci"]): HubPull => ({
  number: 2158,
  url: `${PULL}2158`,
  branch: "armada/14-wire-the-cockpit-s-job-detail-panel-and-job-board",
  ci,
  job: LONG_JOB,
});

const SESSION_MOMENTS: Moment[] = [
  {
    main: { state: "green" },
    pulls: [pull(1830, SESSION_BRANCH, "passed"), longPull("passed"), ...BYSTANDING("passed", "running").pulls],
    jobs: [rowOf(CACHE, "completed_success"), rowOf(DEBOUNCE, "awaiting_review"), ...BYSTANDING("passed", "running").jobs],
  },
  {
    main: { state: "red", red: SESSION_RED },
    pulls: [longPull("waiting_on_main"), ...BYSTANDING("waiting_on_main", "failed").pulls],
    jobs: [rowOf(CACHE, "completed_success"), rowOf(DEBOUNCE, "awaiting_review"), ...BYSTANDING("waiting_on_main", "failed").jobs],
  },
  {
    main: { state: "green" },
    pulls: [longPull("passed"), ...BYSTANDING("passed", "failed").pulls],
    jobs: [rowOf(CACHE, "completed_success"), rowOf(DEBOUNCE, "awaiting_review"), ...BYSTANDING("passed", "failed").jobs],
  },
];

const OTHER_SESSION: HubSession = { id: "session-3d90b1a2", title: "Rework the Cockpit deck cards" };

/** Five landed before and during: the culprit Session's among them, then its fix at the top. */
const SESSION_LANDINGS: Landing[] = [
  { at: 0, number: 1820, branch: branchOf(NOTIFY), owner: { job: hubJob(NOTIFY) }, run: "passed" },
  { at: 0, number: 1824, branch: "nick/ports-cleanup", run: "passed" },
  { at: 0, number: 1826, branch: "armada/session-3d90b1a2", owner: { session: OTHER_SESSION }, run: "passed" },
  { at: 0, number: 1828, branch: branchOf(CACHE), owner: { job: hubJob(CACHE) }, run: "passed" },
  { at: 1, number: 1830, branch: SESSION_BRANCH, owner: { session: SESSION }, run: "failed", failed: "screens_test" },
  { at: 2, number: 1831, branch: `${SESSION_BRANCH}-fix`, owner: { session: SESSION }, run: "passed" },
];

/** The same story told when a Session's pull request is the one that turned main red. */
export function sessionRed(root: string): { first: Partial<BridgeState>; later: Partial<BridgeState>[] } {
  const at = (n: number): Partial<BridgeState> => ({ mergeLines: lineAt(root, n, SESSION_MOMENTS, RECENT_SESSIONS, SESSION_LANDINGS), jobs: SESSION_MOMENTS[n]!.jobs });
  return { first: at(0), later: SESSION_MOMENTS.slice(1).map((_, n) => at(n + 1)) };
}

/** A failed Check's log on main, as the run on the forge printed it. */
const LOGS: Record<string, string[]> = {
  screens_test: [
    " RUN  v3.2.4 /home/user/work/armada/packages/screens",
    "",
    " FAIL  src/manifest-read.test.ts > reads the file again once it changes",
    "AssertionError: expected 'cached' to be 'changed'",
    "",
    " Test Files  1 failed | 64 passed (65)",
    "      Tests  1 failed | 702 passed (703)",
  ],
  desktop_test: [
    " RUN  v3.2.4 /home/user/work/armada/apps/desktop",
    "",
    " FAIL  src/main/resources-poll.test.ts > a reading that stops polls nothing",
    "AssertionError: expected 2 to be 1",
    "",
    " Test Files  1 failed | 91 passed (92)",
    "      Tests  1 failed | 1180 passed (1181)",
  ],
  components_test: [
    " RUN  v4.1.11 /home/user/work/armada/packages/components",
    "",
    " FAIL  src/tokens.test.ts > Theme tokens > the dark ground keeps its contrast",
    "AssertionError: expected 3.9 to be greater than 4.5",
    "",
    " Test Files  1 failed | 186 passed (187)",
    "      Tests  1 failed | 1336 passed (1337)",
  ],
};

LOGS["test-all"] = [
  "Run pnpm test:all",
  "> armada@0.0.0 test:all",
  "> pnpm -r test",
  "ERR_PNPM_RECURSIVE_RUN_FIRST_FAIL packages/screens@0.0.0 test: `vitest run`",
  "Error: Process completed with exit code 1.",
];

/** `followLandCheck`, answered from the logs above: main's run, whole once it has ended. */
export function writingMainsLogs(fleet: FleetHandle): Partial<BridgeApi> {
  return {
    followLandCheck: async (at: LandCheckAt | null) => {
      const lines = at === null ? undefined : LOGS[at.check];
      const landFollowed: FollowedLandLog =
        at === null || lines === undefined
          ? { state: "none" }
          : { state: "following", ...at, fromLine: 1, lines, ended: "finished" };
      fleet.publish({ landFollowed });
    },
  };
}
