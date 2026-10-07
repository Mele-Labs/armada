// Main red while a newer merge's CI is still running, and Recently landed carrying each merge's own
// run on main. Drawn ahead of Fleet as the line's `hub` (23.44); the walk `mainChecksRunning` plays it.
//
//   0  Main is red at #1812, nobody has it: the band offers its two ways. Recently landed shows #1812
//      with its run on main failed, which opens that run's log.
//   1  #1852 merged and its run is going. The red is held: a caution band naming #1852, no buttons.
//   2  #1852's run passed, which clears the red, and #1853's run is going behind it: main is green
//      with checks running, no band.
//   3  #1853 passed too: plain green.
//   4  #1855 merged and broke a job: a new red, with its buttons.
//   5  #1856 merged and its run is going: held again, now naming #1856.
//   6  The run finished red on the same job as #1855's. The red band is back, with its buttons.

import { repository } from "@armada/screens/src/fixtures/build/base";
import type { HubMerged, MainStanding, MergeLine, MergeLineHub } from "@armada/protocol";

import { holding } from "../holding";
import { writingMainsLogs } from "../main-red-hub";
import type { Scenario } from "../moment";

const PULL = "https://git.example/armada/pull/";
const sha = (digit: string) => digit.repeat(40);
const READ_AT = "2026-10-07T10:00:00Z";

type Run = NonNullable<HubMerged["main_run"]>;

const merged = (number: number, branch: string, digit: string, minute: number, main_run?: Run): HubMerged => ({
  number,
  title: branch,
  branch,
  url: `${PULL}${number}`,
  merged_at: `2026-10-07T09:${String(minute).padStart(2, "0")}:00Z`,
  commit: sha(digit),
  ...(main_run === undefined ? {} : { main_run }),
});

const RUNNING: Run = { state: "running" };
const PASSED: Run = { state: "passed" };
const failed = (...jobs: string[]): Run => ({ state: "failed", failed: jobs });

const red = (digit: string, job: string, merge: number, branch: string, checking: MainStanding["checking"] = []): MainStanding => ({
  state: "red",
  commit: checking.length === 0 ? sha(digit) : checking[0]!.commit,
  read_at: READ_AT,
  red_since: READ_AT,
  red_commit: sha(digit),
  failed: [{ name: job, check: job, tests: [`${job === "desktop_test" ? "resources-poll" : "manifest-read"}.test.ts > a reading that stops polls nothing`] }],
  merge: { number: merge, url: `${PULL}${merge}`, branch },
  ...(checking.length === 0 ? {} : { checking }),
});

const checking = (number: number, digit: string, branch: string) => [{ commit: sha(digit), pull_request: { number, url: `${PULL}${number}`, branch } }];

const OLDER = [
  merged(1844, "ci/preview-test-and-stranded", "4", 20, PASSED),
  merged(1840, "docs/connection-states", "3", 10, PASSED),
];

const M1812 = merged(1812, "nick/ports-cleanup", "1", 40, failed("desktop_test"));
const M1852 = (run: Run) => merged(1852, "fleet/flaky-asked-run-test", "2", 50, run);
const M1853 = (run: Run) => merged(1853, "docs/reflect-bridge-micro-apps", "3", 52, run);
const M1855 = (run: Run) => merged(1855, "studio/zone-proposal", "5", 58, run);
const M1856 = (run: Run) => merged(1856, "fleet/pause-markers", "6", 59, run);
const green = (digit: string): MainStanding => ({ state: "green", commit: sha(digit), read_at: READ_AT });
const greenChecking = (digit: string, ...running: [number, string, string][]): MainStanding => ({
  ...green(digit),
  checking: running.flatMap(([number, d, branch]) => checking(number, d, branch)),
});

/** The hub at each moment. */
const MOMENTS: MergeLineHub[] = [
  { main: red("1", "desktop_test", 1812, "nick/ports-cleanup"), merged: [M1812, ...OLDER] },
  {
    main: red("1", "desktop_test", 1812, "nick/ports-cleanup", checking(1852, "2", "fleet/flaky-asked-run-test")),
    merged: [M1852(RUNNING), M1812, ...OLDER],
  },
  // #1852 passed, which clears the red, and #1853 is running behind it.
  {
    main: greenChecking("2", [1853, "3", "docs/reflect-bridge-micro-apps"]),
    merged: [M1853(RUNNING), M1852(PASSED), M1812, ...OLDER],
  },
  { main: green("3"), merged: [M1853(PASSED), M1852(PASSED), M1812, ...OLDER] },
  {
    main: red("5", "screens_test", 1855, "studio/zone-proposal"),
    merged: [M1855(failed("screens_test")), M1853(PASSED), M1852(PASSED), M1812],
  },
  {
    main: red("5", "screens_test", 1855, "studio/zone-proposal", checking(1856, "6", "fleet/pause-markers")),
    merged: [M1856(RUNNING), M1855(failed("screens_test")), M1853(PASSED), M1852(PASSED)],
  },
  {
    main: red("6", "screens_test", 1855, "studio/zone-proposal"),
    merged: [M1856(failed("screens_test")), M1855(failed("screens_test")), M1853(PASSED), M1852(PASSED)],
  },
];

function at(root: string, n: number) {
  return { mergeLines: { lines: [{ root, line: [], off: [], landed: [], sent_back: [], hub: MOMENTS[n]! } as MergeLine] } };
}

function checksRunning(): Scenario {
  const root = repository().root;
  const base = holding("main-checks-running", "Main red, held while a newer merge's checks run, then green and red again", [], {
    alsoServed: [repository()],
  });
  return { ...base, state: { ...base.state, ...at(root, 0) }, later: MOMENTS.slice(1).map((_, n) => at(root, n + 1)), behaves: writingMainsLogs };
}

export const s181MainChecksRunning: Scenario = checksRunning();
