// A merge-line turn with a Check failing in it, one moment at a time: what the owner is told while
// the turn runs on. Three repositories, each at its own place in the story, and the Job's own log
// beside them. **Drawn ahead of Fleet**: a line's `notice` and the Job's two notes are not served
// yet, and nothing here reaches the wire. `the-failed-check-mid-turn.ts` walks it.
//
//   armada   a batch of three. screens_test fails, rerun fails: a heads-up to all three. The split
//            names one. The turn ends and that one is sent back; the alert is the verdict.
//   notes    one branch. The same Check fails: the branch is told at once, then sent back.
//   scratch  one branch. Told, then the same Check is red on main too: nobody's, the turn holds.

import type { FollowedLandLog, Journalled, LandCheckAt, MergeLine, MergeLineRow, Noted } from "@armada/protocol";
import type { MergeLineNotice } from "@armada/components";
import { note } from "@armada/screens/src/fixtures/build/base";

import type { BridgeApi } from "../../../shared/api";
import type { BridgeState } from "../../../shared/bridge";
import type { FleetHandle } from "./moment";

const TURN = "docs/wire-lock-signed";
const AT_FAULT = "fleet/gate-policy-every-run";
const FIRST = "docs/wire-lock-signed";
/** The member whose row carries the turn's Checks, and whose name opens their logs. */
const CARRIER = "worktree-agent-aef3c24792026e2c3";
const WAITING = "fleet/read-in-cluster-membership";
const ALONE = "notes/reading-list-tags";
const HELD = "scratch/try-ports";

type CheckState = "waiting" | "running" | "passed" | "failed";

/** A turn's Checks as they stand, in the order it runs them. */
function checks(...states: CheckState[]): { name: string; state: string }[] {
  const names = ["build", "typecheck", "screens_test", "desktop_test", "components_test"];
  return states.map((state, at) => ({ name: names[at]!, state }));
}

const row = (place: number, branch: string, over: Partial<MergeLineRow> = {}): MergeLineRow => ({
  place,
  branch,
  state: "waiting",
  ...over,
});

/** The batch's members in one turn, the second one carrying the Checks as a turn's row does. */
function batch(states: CheckState[]): MergeLineRow[] {
  return [
    row(1, FIRST, { state: "gating", batch: TURN, doing: "reading verify-foundations against main" }),
    row(2, CARRIER, { state: "gating", batch: TURN, checks: checks(...states) }),
    row(3, AT_FAULT, { state: "gating", batch: TURN, doing: "reading verify-foundations against main" }),
    row(4, WAITING),
  ];
}

const alone = (branch: string, states: CheckState[], test: string): MergeLineRow =>
  row(1, branch, { state: "gating", checks: states.map((state, at) => ({ name: ["build", test, "typecheck"][at]!, state })) });

type Moment = {
  armada: { line: MergeLineRow[]; sent_back?: MergeLineRow[]; notice?: MergeLineNotice };
  notes: { line: MergeLineRow[]; sent_back?: MergeLineRow[]; notice?: MergeLineNotice };
  scratch: { line: MergeLineRow[]; notice?: MergeLineNotice };
  /** What Fleet wrote to the Job's own log by now. */
  notes_in_log: number;
};

const SCREENS = "screens_test";

/** Each moment of the story. The first is what a window opens on. */
const MOMENTS: Moment[] = [
  {
    armada: { line: batch(["passed", "passed", "running", "waiting", "waiting"]) },
    notes: { line: [alone(ALONE, ["passed", "running", "waiting"], "notes_test")] },
    scratch: { line: [alone(HELD, ["passed", "running", "waiting"], "ports_test")] },
    notes_in_log: 0,
  },
  {
    armada: {
      line: batch(["passed", "passed", "failed", "running", "waiting"]),
      notice: { kind: "batch", check: SCREENS, branch: CARRIER, branches: [FIRST, CARRIER, AT_FAULT] },
    },
    notes: {
      line: [alone(ALONE, ["passed", "failed", "running"], "notes_test")],
      notice: { kind: "branch", check: "notes_test", branch: ALONE },
    },
    scratch: {
      line: [alone(HELD, ["passed", "failed", "running"], "ports_test")],
      notice: { kind: "branch", check: "ports_test", branch: HELD },
    },
    notes_in_log: 1,
  },
  {
    armada: {
      line: batch(["passed", "passed", "failed", "passed", "running"]),
      notice: { kind: "branch", check: SCREENS, branch: AT_FAULT },
    },
    notes: {
      line: [alone(ALONE, ["passed", "failed", "passed"], "notes_test")],
      notice: { kind: "branch", check: "notes_test", branch: ALONE },
    },
    scratch: {
      line: [alone(HELD, ["passed", "failed", "waiting"], "ports_test")],
      notice: { kind: "main", check: "ports_test", branch: HELD },
    },
    notes_in_log: 2,
  },
  {
    armada: {
      line: [
        row(1, FIRST, { state: "preparing", doing: "merging main (c527f60e09) into docs/wire-lock-signed" }),
        row(2, WAITING),
      ],
      sent_back: [
        {
          branch: AT_FAULT,
          state: "red",
          failed: [SCREENS],
          checks: checks("passed", "passed", "failed", "passed", "passed"),
        },
      ],
      notice: { kind: "sent", check: SCREENS, branch: AT_FAULT },
    },
    notes: {
      line: [],
      sent_back: [
        {
          branch: ALONE,
          state: "red",
          failed: ["notes_test"],
          checks: [
            { name: "build", state: "passed" },
            { name: "notes_test", state: "failed" },
            { name: "typecheck", state: "passed" },
          ],
        },
      ],
      notice: { kind: "sent", check: "notes_test", branch: ALONE },
    },
    scratch: {
      line: [alone(HELD, ["passed", "failed", "waiting"], "ports_test")],
      notice: { kind: "main", check: "ports_test", branch: HELD },
    },
    notes_in_log: 2,
  },
];

/** The three lines at one moment, with the rows each repository already had apart from the turn. */
function linesAt(at: number, roots: TurnRoots): MergeLine[] {
  const moment = MOMENTS[at]!;
  const line = (root: string, one: Moment["armada"]): MergeLine => ({
    root,
    line: one.line,
    off: [],
    landed: [],
    sent_back: one.sent_back ?? [],
    ...(one.notice === undefined ? {} : { notice: one.notice }),
  });
  return [
    line(roots.armada, moment.armada),
    line(roots.notes, moment.notes),
    line(roots.scratch, moment.scratch),
    queued(roots.bridge),
  ];
}

/** What Fleet writes to the Job's own log as the turn goes: told it, then told whose it was. */
const WRITTEN: Noted[] = (() => {
  return [
    note("2026-10-05T11:42:10Z", "screens_test failed in the merge line turn", {
      level: "warn",
      fields: [
        { name: "Check", value: "screens_test" },
        { name: "Rerun", value: "failed" },
        { name: "Main", value: "green" },
      ],
    }),
    note("2026-10-05T11:44:31Z", "screens_test is fleet/gate-policy-every-run's", {
      fields: [
        { name: "Check", value: "screens_test" },
        { name: "Branch", value: AT_FAULT },
      ],
    }),
  ];
})();

/** The Job's log at a moment: what it held, and what the turn has written since. */
export function journalAt(was: Journalled, at: number): Journalled {
  if (!("log" in was)) return was;
  const kept = was.log.notes.filter((one) => !WRITTEN.some((added) => added.msg === one.msg));
  return { ...was, log: { ...was.log, notes: [...kept, ...WRITTEN.slice(0, MOMENTS[at]!.notes_in_log)] } };
}

export type TurnRoots = { armada: string; notes: string; scratch: string; bridge: string };

/**
 * A line whose turn took two of four and left the rest waiting, in the places they joined at: the
 * first kept its place after a red, so it sits idle ahead of the turn that is running. The panel
 * draws the turn first, then the waiting in the order the next turn takes them, each with its reason.
 */
function queued(root: string): MergeLine {
  const running = (place: number, branch: string, over: Partial<MergeLineRow> = {}) =>
    row(place, branch, { state: "gating", batch: "canvas/gates-on-the-spine", ...over });
  const waiting = (place: number, branch: string, why?: string) =>
    ({ ...row(place, branch), ...(why === undefined ? {} : { why }) }) as MergeLineRow;
  return {
    root,
    line: [
      waiting(1, "fleet/stranded-slot-rescue", "kept"),
      running(2, "canvas/gates-on-the-spine", {
        checks: [
          { name: "build", state: "passed" },
          { name: "typecheck", state: "running" },
          { name: "desktop_test", state: "waiting" },
        ],
      }),
      running(3, "fix/overview-fallback-and-reads", { doing: "reading verify-foundations against main" }),
      waiting(4, "fix/dispatch-did-not-answer"),
      waiting(5, "fix/pulse-log-clash", "member"),
      waiting(6, "fix/stale-base-read", "main"),
      waiting(7, "bridge/late-joiner", "late"),
    ],
    off: [],
    landed: [],
    sent_back: [],
  };
}

/** The state at the first moment, and each moment after as the change a walk publishes. */
export function failingTurn(
  roots: TurnRoots,
  journal: Journalled,
): { first: Partial<BridgeState>; later: Partial<BridgeState>[] } {
  const at = (n: number): Partial<BridgeState> => ({
    mergeLines: { lines: linesAt(n, roots) },
    journalled: journalAt(journal, n),
  });
  return { first: at(0), later: MOMENTS.slice(1).map((_, n) => at(n + 1)) };
}

/** A Check's log, finished: the line the walk opens from the alert, the strip and the row alike. */
const LOGS: Record<string, string[]> = {
  screens_test: [
    " RUN  v3.2.4 /Users/user/armada/packages/screens",
    "",
    " FAIL  src/merge-line.test.ts > folds a line's failed Check onto the panel",
    "AssertionError: expected undefined to be 'branch'",
    "",
    " Test Files  1 failed | 64 passed (65)",
    "      Tests  1 failed | 702 passed (703)",
  ],
  ports_test: [
    " RUN  v3.2.4 /Users/user/scratch",
    "",
    " FAIL  src/ports.test.ts > a port in use is skipped",
    "AssertionError: expected 7878 to be 7879",
    "",
    " Test Files  1 failed | 3 passed (4)",
  ],
  notes_test: [
    " RUN  v3.2.4 /Users/user/notes",
    "",
    " FAIL  src/tags.test.ts > a tag with a slash keeps its folder",
    "AssertionError: expected 'reading' to be 'reading/list'",
    "",
    " Test Files  1 failed | 11 passed (12)",
  ],
};

/** `followLandCheck`, answered from the logs above. */
export function writingTheFailedLogs(fleet: FleetHandle): Partial<BridgeApi> {
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
