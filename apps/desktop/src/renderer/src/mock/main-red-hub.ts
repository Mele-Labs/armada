// Main going red on CI, three times, one moment at a time: what the owner sees and does. The hub's
// head and its pull requests are what Fleet serves as the line's `hub`. `main-goes-red.ts` walks it.
//
//   1  A pull request merges and main goes red. Nobody holds it, and the owner is asked.
//   2  A person merges a pull request by hand and main goes red. The owner is asked.
//   3  A person's own pull request turns it red. The owner is asked and dispatches a new Session.
//
// Each ends with the fix landing: main green.

import type { HubPull, HubSession, MainRed, MainState } from "@armada/components";
import type { Session } from "@armada/screens/src/draft/sessions";
import type { FixingElsewhere, FollowedLandLog, HubMerged, HubPullRequest, LandCheckAt, MainStanding, MergeLine, MergeLineHub, MergeLines } from "@armada/protocol";

import type { BridgeApi } from "../../../shared/api";
import type { BridgeState } from "../../../shared/bridge";
import type { FleetHandle } from "./moment";

const PULL = "https://git.example/armada/pull/";

/** The branches of the pull requests in the story. */
const CACHE = "armada/cache";
const DEBOUNCE = "armada/debounce";
const NOTIFY = "armada/notify";
const ZONE = "armada/zone";
const FIX = "armada/fix-main";

/** The three reds: what failed, its test, and the pull request that merged it. */
const FIRST: MainRed = {
  check: "screens_test",
  test: "manifest-read.test.ts > reads the file again once it changes",
  merge: { number: 1812, url: `${PULL}1812`, branch: CACHE },
};
const SECOND: MainRed = {
  check: "desktop_test",
  test: "resources-poll.test.ts > a reading that stops polls nothing",
  merge: { number: 1816, url: `${PULL}1816`, branch: DEBOUNCE },
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
  merge: { number: 1814, url: `${PULL}1814`, branch: "studio/zone-proposal" },
};

const pull = (number: number, branch: string, ci: HubPull["ci"]): HubPull => ({
  number,
  url: `${PULL}${number}`,
  branch,
  ci,
});

type Moment = {
  main: MainState;
  /** The pull requests open now, by how each one's `ci` stands. */
  pulls: HubPull[];
};

/** The pull requests that are not part of the story. */
const BYSTANDING = (zone: HubPull["ci"], pause: HubPull["ci"]): { pulls: HubPull[] } => ({
  pulls: [pull(1814, "studio/zone-proposal", zone), pull(1819, "fleet/pause-markers", pause)],
});

const PERSON = pull(1815, "nick/theme-tokens", "passed");

/** Each moment of the story. The first is what a window opens on. */
const MOMENTS: Moment[] = [
  // 0  Green, with two pull requests about to merge.
  {
    main: { state: "green" },
    pulls: [pull(1812, CACHE, "passed"), pull(1816, DEBOUNCE, "passed"), PERSON, ...BYSTANDING("passed", "running").pulls],
  },
  // 1  #1812 merged and main is red.
  {
    main: { state: "red", red: FIRST },
    pulls: [pull(1816, DEBOUNCE, "waiting_on_main"), { ...PERSON, ci: "waiting_on_main" }, ...BYSTANDING("waiting_on_main", "failed").pulls],
  },
  // 2  Its fix landed in #1821: green.
  {
    main: { state: "green" },
    pulls: [pull(1816, DEBOUNCE, "passed"), PERSON, ...BYSTANDING("passed", "failed").pulls],
  },
  // 3  A person merged #1816 on the forge, and main is red.
  {
    main: { state: "red", red: SECOND },
    pulls: [{ ...PERSON, ci: "waiting_on_main" }, ...BYSTANDING("waiting_on_main", "failed").pulls],
  },
  // 4  #1822 landed.
  {
    main: { state: "green" },
    pulls: [PERSON, ...BYSTANDING("passed", "failed").pulls],
  },
  // 5  A person's own pull request turned it red.
  {
    main: { state: "red", red: THIRD },
    pulls: BYSTANDING("waiting_on_main", "failed").pulls,
  },
  // 6  #1823 landed.
  {
    main: { state: "green" },
    pulls: BYSTANDING("passed", "failed").pulls,
  },
  // 7  #1814 merged. Its CI job maps to no Check, and no test could be read.
  {
    main: { state: "red", red: FOURTH },
    pulls: [pull(1819, "fleet/pause-markers", "failed")],
  },
  // 8  #1824 landed.
  {
    main: { state: "green" },
    pulls: [pull(1819, "fleet/pause-markers", "failed")],
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
});

/**
 * A pull request that merged, as the forge lists it, with the CI run on its merge commit. `at` is the
 * moment it lands; a list is every one that has, newest first, up to what the panel keeps.
 */
type Landing = { at: number; number: number; branch: string; run: "passed" | "failed"; failed?: string };

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
      main_run: { state: one.run, ...(one.failed === undefined ? {} : { failed: [one.failed] }) },
    }));
}

/** The line a moment serves for the repository, with the hub on it. */
function lineAt(root: string, at: number, moments: readonly Moment[] = MOMENTS, landings: readonly Landing[] = LANDINGS): MergeLines {
  const { main, pulls } = moments[at]!;
  const hub: MergeLineHub = {
    main: standing(main),
    pull_requests: pulls.map(requestOf),
    ...(main.state === "red" && main.takenBySession !== undefined ? { fixing_session: main.takenBySession } : {}),
    ...(main.state === "red" && main.elsewhere !== undefined ? { fixing_elsewhere: main.elsewhere } : {}),
    merged: mergedAt(landings, at),
  };
  return { lines: [{ root, line: [], off: [], landed: [], sent_back: [], hub } as MergeLine] };
}

/** What merged before the story and during it, in the order the moments land it. */
const LANDINGS: Landing[] = [
  { at: 0, number: 1799, branch: "nick/ports-cleanup", run: "passed" },
  { at: 0, number: 1803, branch: NOTIFY, run: "passed" },
  { at: 0, number: 1808, branch: "fleet/pause-markers-read", run: "passed" },
  { at: 1, number: 1812, branch: CACHE, run: "failed", failed: "screens_test" },
  { at: 2, number: 1821, branch: `${CACHE}-fix`, run: "passed" },
  { at: 3, number: 1816, branch: DEBOUNCE, run: "failed", failed: "desktop_test" },
  { at: 4, number: 1822, branch: `${DEBOUNCE}-fix`, run: "passed" },
  { at: 5, number: 1815, branch: "nick/theme-tokens", run: "failed", failed: "components_test" },
  { at: 6, number: 1823, branch: FIX, run: "passed" },
  { at: 7, number: 1814, branch: "studio/zone-proposal", run: "failed", failed: "test-all" },
  { at: 8, number: 1824, branch: `${ZONE}-fix`, run: "passed" },
];

/** The state at the first moment, and each moment after as the change a walk publishes. */
export function mainRed(root: string): { first: Partial<BridgeState>; later: Partial<BridgeState>[] } {
  const at = (n: number): Partial<BridgeState> => ({ mergeLines: lineAt(root, n) });
  return { first: at(0), later: MOMENTS.slice(1).map((_, n) => at(n + 1)) };
}

// A Session's pull request turned main red. Fleet names the Session that owns the pull request.
// `main-red-session` walks it.
//
//   0  The Session's pull request is open and green.
//   1  It merged and main is red: the head names the Session, and the owner is asked.
//   2  The owner sent the work back to the Session, and its fix landed.

const SESSION: HubSession = { id: "session-c279e883", title: "Show who merged on the head" };
const SESSION_BRANCH = "armada/session-c279e883";

const SESSION_RED: MainRed = {
  check: "screens_test",
  test: "merge-line.test.ts > folds a Session's pull request onto the head",
  merge: { number: 1830, url: `${PULL}1830`, branch: SESSION_BRANCH, session: SESSION },
};

/** A pull request whose branch is longer than its row: it is cut short. */
const longPull = (ci: HubPull["ci"]): HubPull => ({
  number: 2158,
  url: `${PULL}2158`,
  branch: "armada/14-wire-the-cockpit-s-session-panel-and-session-view",
  ci,
});

const SESSION_MOMENTS: Moment[] = [
  {
    main: { state: "green" },
    pulls: [pull(1830, SESSION_BRANCH, "passed"), longPull("passed"), ...BYSTANDING("passed", "running").pulls],
  },
  {
    main: { state: "red", red: SESSION_RED },
    pulls: [longPull("waiting_on_main"), ...BYSTANDING("waiting_on_main", "failed").pulls],
  },
  {
    main: { state: "green" },
    pulls: [longPull("passed"), ...BYSTANDING("passed", "failed").pulls],
  },
];

// The same red, with the Session that merged it holding the red: Fleet woke it and recorded its take, so
// the band names it in the Fixing row and offers nothing. `main-red-session-fixing` walks it.
//
//   0  The Session's pull request is open and green.
//   1  It merged and main is red. Fleet woke the Session: it is in the Fixing row.
//   2  The Session's fix landed.

const SESSION_FIXING_MOMENTS: Moment[] = [
  SESSION_MOMENTS[0]!,
  { ...SESSION_MOMENTS[1]!, main: { state: "red", red: SESSION_RED, takenBySession: SESSION } },
  SESSION_MOMENTS[2]!,
];

/** The Sessions the window holds, the one that merged among them: the registry owns the chips and fills the picker. */
const pull_request = (number: number, branch: string, title: string) =>
  ({ kind: "pull_request", number, title, branch, address: `${PULL}${number}`, checks: { state: "passed" }, state: "merged", auto: false }) as const;
export const SESSION_REGISTRY: Session[] = [
  {
    id: "session-3d90b1a2",
    title: "Rework the Cockpit deck cards",
    turn: { state: "idle" },
    lastTurnAt: "2026-10-12T10:02:00Z",
    rows: [],
    attachments: [{ kind: "branch", name: "armada/session-3d90b1a2", slot: 3 }, pull_request(1826, "armada/session-3d90b1a2", "Rework the Cockpit deck cards")],
  },
  {
    id: SESSION.id,
    title: SESSION.title,
    turn: { state: "idle" },
    lastTurnAt: "2026-10-12T10:20:00Z",
    rows: [],
    attachments: [
      { kind: "branch", name: SESSION_BRANCH, slot: 2 },
      pull_request(1830, SESSION_BRANCH, SESSION.title),
      pull_request(1831, `${SESSION_BRANCH}-fix`, SESSION.title),
    ],
  },
];

/** Five landed before and during: the culprit Session's among them, then its fix at the top. */
const SESSION_LANDINGS: Landing[] = [
  { at: 0, number: 1820, branch: NOTIFY, run: "passed" },
  { at: 0, number: 1824, branch: "nick/ports-cleanup", run: "passed" },
  { at: 0, number: 1826, branch: "armada/session-3d90b1a2", run: "passed" },
  { at: 0, number: 1828, branch: CACHE, run: "passed" },
  { at: 1, number: 1830, branch: SESSION_BRANCH, run: "failed", failed: "screens_test" },
  { at: 2, number: 1831, branch: `${SESSION_BRANCH}-fix`, run: "passed" },
];

/** The same story told when a Session's pull request is the one that turned main red. */
export function sessionRed(root: string): { first: Partial<BridgeState>; later: Partial<BridgeState>[] } {
  const at = (n: number): Partial<BridgeState> => ({ mergeLines: lineAt(root, n, SESSION_MOMENTS, SESSION_LANDINGS) });
  return { first: at(0), later: SESSION_MOMENTS.slice(1).map((_, n) => at(n + 1)) };
}

/** The red a Session holds, as a wake leaves it. */
export function sessionFixing(root: string): { first: Partial<BridgeState>; later: Partial<BridgeState>[] } {
  const at = (n: number): Partial<BridgeState> => ({ mergeLines: lineAt(root, n, SESSION_FIXING_MOMENTS, SESSION_LANDINGS) });
  return { first: at(0), later: SESSION_FIXING_MOMENTS.slice(1).map((_, n) => at(n + 1)) };
}

// Main red, and the claim on it held on another machine. A person's pull request turned it red, so no
// Session of ours was watching it. `red-held-elsewhere` walks it.
//
//   0  Another machine's Session has the red, claimed 12 minutes ago.
//   1  The red is held by a Session on another machine, claimed 3 hours ago: stale.
//   2  The owner took it over, and a Session here is working on it.
//   3  Its fix landed: main is green.

const MINUTE = 60_000;
const claimed = (minutes: number): string => new Date(Date.now() - minutes * MINUTE).toISOString();

/** The moments, with each claim's age counted from when the story is built. */
function elsewhereMoments(): Moment[] {
  const stranded = BYSTANDING("waiting_on_main", "failed");
  const held = (elsewhere: FixingElsewhere): Moment => ({ main: { state: "red", red: THIRD, elsewhere }, pulls: stranded.pulls });
  return [
    held({ machine: "work-mac-mini", holder: { kind: "session", id: "7be40d15" }, since: claimed(12) }),
    held({ machine: "nick-mbp", holder: { kind: "session", id: "9c74d82e" }, since: claimed(3 * 60) }),
    { main: { state: "red", red: THIRD, takenBySession: SESSION }, pulls: stranded.pulls },
    { main: { state: "green" }, pulls: BYSTANDING("passed", "failed").pulls },
  ];
}

const ELSEWHERE_LANDINGS: Landing[] = [
  { at: 0, number: 1799, branch: "nick/ports-cleanup", run: "passed" },
  { at: 0, number: 1815, branch: "nick/theme-tokens", run: "failed", failed: "components_test" },
  { at: 3, number: 1823, branch: FIX, run: "passed" },
];

/** The same red, claimed on another machine and then taken over. */
export function redHeldElsewhere(root: string): { first: Partial<BridgeState>; later: Partial<BridgeState>[] } {
  const moments = elsewhereMoments();
  const at = (n: number): Partial<BridgeState> => ({ mergeLines: lineAt(root, n, moments, ELSEWHERE_LANDINGS) });
  return { first: at(0), later: moments.slice(1).map((_, n) => at(n + 1)) };
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
