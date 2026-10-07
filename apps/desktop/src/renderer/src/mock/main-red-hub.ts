// Main going red on CI, three times, one moment at a time: what the owner sees and does. The hub's
// head and its pull requests are what Fleet serves as the line's `hub` since 23.41. **Not served
// yet**: a Job's part in the red, which rides on its row as `fixes_main`, and the two ways to hand
// the red to a Job, which ride on the line as `fix_offered`, the way the failed-Check mock carried
// `notice`. `main-goes-red.ts` walks it.
//
//   1  The Job's own pull request merges and main goes red. That Job was watching its landing, so it
//      takes the red itself and nobody is asked.
//   2  A person merges a Job's pull request by hand, so nothing was watching it. The owner is asked,
//      and sends the work back to that Job, listed first among the recent ones.
//   3  A person's own pull request turns it red. The owner is asked and dispatches a new Job.
//
// Each ends with the fix landing: main green, and the Job that fixed it says so on its row.

import type { HubJob, HubPull, MainRed, MainState, RecentJob } from "@armada/components";
import type { FollowedLandLog, HubPullRequest, JobSummary, LandCheckAt, MainStanding, MergeLine, MergeLineHub, MergeLines } from "@armada/protocol";
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
const recent = (one: JobFixture): RecentJob => ({ ...hubJob(one), branch: branchOf(one) });

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
  /** The Jobs work can be sent back to, newest first. */
  recent: RecentJob[];
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
    recent: [recent(NOTIFY)],
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
    recent: [recent(NOTIFY)],
    jobs: [rowOf(CACHE, "running", fixing(FIRST)), rowOf(DEBOUNCE, "awaiting_review"), ...BYSTANDING("waiting_on_main", "failed").jobs],
  },
  // 2  Its fix landed in #1821: green, and the Job says it fixed it.
  {
    main: { state: "green" },
    pulls: [pull(1816, branchOf(DEBOUNCE), "passed", DEBOUNCE), PERSON, ...BYSTANDING("passed", "failed").pulls],
    recent: [recent(CACHE), recent(NOTIFY)],
    jobs: [rowOf(CACHE, "completed_success", fixed(FIRST, 1821)), rowOf(DEBOUNCE, "awaiting_review"), ...BYSTANDING("passed", "failed").jobs],
  },
  // 3  A person merged the Job's pull request on the forge, so nothing was watching it.
  {
    main: { state: "red", red: SECOND },
    pulls: [{ ...PERSON, ci: "waiting_on_main" }, ...BYSTANDING("waiting_on_main", "failed").pulls],
    recent: [recent(CACHE), recent(DEBOUNCE), recent(NOTIFY)],
    jobs: [rowOf(CACHE, "completed_success", fixed(FIRST, 1821)), rowOf(DEBOUNCE, "completed_success"), ...BYSTANDING("waiting_on_main", "failed").jobs],
  },
  // 4  The owner sent the work back to it, and it is running.
  {
    main: { state: "red", red: SECOND, taken: hubJob(DEBOUNCE) },
    pulls: [{ ...PERSON, ci: "waiting_on_main" }, ...BYSTANDING("waiting_on_main", "failed").pulls],
    recent: [recent(CACHE), recent(DEBOUNCE), recent(NOTIFY)],
    jobs: [rowOf(CACHE, "completed_success", fixed(FIRST, 1821)), rowOf(DEBOUNCE, "running", fixing(SECOND)), ...BYSTANDING("waiting_on_main", "failed").jobs],
  },
  // 5  #1822 landed.
  {
    main: { state: "green" },
    pulls: [PERSON, ...BYSTANDING("passed", "failed").pulls],
    recent: [recent(DEBOUNCE), recent(CACHE), recent(NOTIFY)],
    jobs: [rowOf(CACHE, "completed_success", fixed(FIRST, 1821)), rowOf(DEBOUNCE, "completed_success", fixed(SECOND, 1822)), ...BYSTANDING("passed", "failed").jobs],
  },
  // 6  A person's own pull request turned it red.
  {
    main: { state: "red", red: THIRD },
    pulls: BYSTANDING("waiting_on_main", "failed").pulls,
    recent: [recent(DEBOUNCE), recent(CACHE), recent(NOTIFY)],
    jobs: [rowOf(CACHE, "completed_success", fixed(FIRST, 1821)), rowOf(DEBOUNCE, "completed_success", fixed(SECOND, 1822)), ...BYSTANDING("waiting_on_main", "failed").jobs],
  },
  // 7  The owner dispatched a new Job, and it is running.
  {
    main: { state: "red", red: THIRD, taken: hubJob(FIX) },
    pulls: BYSTANDING("waiting_on_main", "failed").pulls,
    recent: [recent(DEBOUNCE), recent(CACHE), recent(NOTIFY)],
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
    recent: [recent(FIX), recent(DEBOUNCE), recent(CACHE), recent(NOTIFY)],
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
    recent: [recent(FIX), recent(DEBOUNCE), recent(CACHE), recent(NOTIFY)],
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
    recent: [recent(ZONE), recent(FIX), recent(DEBOUNCE), recent(CACHE), recent(NOTIFY)],
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
    ...(merge === undefined ? {} : { merge: { number: merge.number, url: merge.url, branch: merge.branch, ...(merge.job === undefined ? {} : { job: merge.job }) } }),
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

/** The line a moment serves for the repository, with the hub on it. */
function lineAt(root: string, at: number): MergeLines {
  const { main, pulls, recent: others } = MOMENTS[at]!;
  const hub: MergeLineHub = {
    main: standing(main),
    pull_requests: pulls.map(requestOf),
    ...(main.state === "red" && main.taken !== undefined ? { fixing: main.taken } : {}),
  };
  return { lines: [{ root, line: [], off: [], landed: [], sent_back: [], hub, fix_offered: { recent: others } } as MergeLine] };
}

/** The state at the first moment, and each moment after as the change a walk publishes. */
export function mainRed(root: string): { first: Partial<BridgeState>; later: Partial<BridgeState>[] } {
  const at = (n: number): Partial<BridgeState> => ({ mergeLines: lineAt(root, n), jobs: MOMENTS[n]!.jobs });
  return { first: at(0), later: MOMENTS.slice(1).map((_, n) => at(n + 1)) };
}

/** A failed Check's log on main, as the run on the forge printed it. */
const LOGS: Record<string, string[]> = {
  screens_test: [
    " RUN  v3.2.4 /home/runner/work/armada/packages/screens",
    "",
    " FAIL  src/manifest-read.test.ts > reads the file again once it changes",
    "AssertionError: expected 'cached' to be 'changed'",
    "",
    " Test Files  1 failed | 64 passed (65)",
    "      Tests  1 failed | 702 passed (703)",
  ],
  desktop_test: [
    " RUN  v3.2.4 /home/runner/work/armada/apps/desktop",
    "",
    " FAIL  src/main/resources-poll.test.ts > a reading that stops polls nothing",
    "AssertionError: expected 2 to be 1",
    "",
    " Test Files  1 failed | 91 passed (92)",
    "      Tests  1 failed | 1180 passed (1181)",
  ],
  components_test: [
    " RUN  v4.1.11 /home/runner/work/armada/packages/components",
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
