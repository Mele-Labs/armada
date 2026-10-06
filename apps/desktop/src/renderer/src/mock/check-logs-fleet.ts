// A mock Fleet whose Checks write their logs as they run: group three's boundary on the arc's Job,
// and the merge line's turn. Each socket a log panel opens answers here the way main publishes one
// — the lines already written, then one line at a time while the Check is running, and `finished`
// at once for a Check that has ended. `check-logs`, the scenario the walk of the same change plays.

import type { FollowedLandLog, FollowedLog, LandCheckAt } from "@armada/protocol";
import { LIVE_LOGS } from "@armada/screens/src/fixtures/build/arc-checking";

import type { BridgeApi } from "../../../shared/api";
import type { FleetHandle } from "./moment";

/**
 * How far apart a running Check's lines arrive: inside a walk step's five-second wait. **A walk
 * that reads two of them waits out six seconds of the clock**, which is what put it past its
 * budget on a loaded runner, so the suite that plays walks shortens it: `play-walks.ts`.
 */
export const pace = { arrivingMs: 2_000 };

/** A log: what it held when the panel opened, and what the Check writes after. Nothing after is ended. */
type Played = { lines: readonly string[]; arriving: readonly string[] };

const RUN = " RUN  v3.2.4 /Users/user/armada/packages/screens";

/** The arc Job's live logs, by the file's own name. */
const JOB_LOGS: Record<string, Played> = {
  [name(LIVE_LOGS.test)]: {
    lines: ["   Compiling armada v0.0.0", "    Finished `test` profile in 41.2s", "     Summary 4154 tests run: 4154 passed"],
    arriving: [],
  },
  [name(LIVE_LOGS.typecheck)]: { lines: ["$ tsc -b --force"], arriving: [] },
  [name(LIVE_LOGS.screens_test)]: {
    lines: [RUN, "", " ✓ src/board.test.ts (22 tests) 31ms", " ✓ src/plan-board.test.ts (31 tests) 58ms"],
    arriving: [
      " ✓ src/merge-line.test.ts (12 tests) 9ms",
      " ✓ src/running.test.tsx (8 tests) 412ms",
      " ✓ src/check-log-sheet.test.tsx (6 tests) 188ms",
      " ✓ src/record.test.ts (40 tests) 77ms",
    ],
  },
};

/** The merge line's logs, by branch and Check. */
const LAND_LOGS: Record<string, Played> = {
  "worktree-agent-aef3c24792026e2c3 screens_test": {
    lines: [RUN, "", " ✓ src/merge-line.test.ts (12 tests) 9ms"],
    arriving: [" ✓ src/sheets.test.ts (9 tests) 14ms", " ✓ src/Row.test.ts (18 tests) 21ms", " ✓ src/notes.test.ts (7 tests) 6ms"],
  },
  "worktree-agent-aef3c24792026e2c3 build": {
    lines: ["   Compiling fleet v0.0.0", "    Finished `dev` profile in 38.0s"],
    arriving: [],
  },
  "fleet/pulse-log-rows desktop_test": {
    lines: [
      " RUN  v3.2.4 /Users/user/armada/apps/desktop",
      "",
      " FAIL  src/main/resources-poll.test.ts > a reading that stops polls nothing",
      "AssertionError: expected 2 to be 1",
      "",
      " Test Files  1 failed | 64 passed (65)",
      "      Tests  1 failed | 702 passed (703)",
    ],
    arriving: [],
  },
};

/** `followCheckOutput` and `followLandCheck`, answered from the logs above. */
export function writingLogs(fleet: FleetHandle): Partial<BridgeApi> {
  let timers: ReturnType<typeof setTimeout>[] = [];
  const stop = () => {
    timers.forEach(clearTimeout);
    timers = [];
  };
  /** What the log held, then each line as it arrives. Never ends while lines are still to come. */
  function play(log: Played, show: (lines: string[], ended: string | undefined) => void): void {
    show([...log.lines], log.arriving.length === 0 ? "finished" : undefined);
    log.arriving.forEach((_, n) => {
      timers.push(setTimeout(() => show([...log.lines, ...log.arriving.slice(0, n + 1)], undefined), pace.arrivingMs * (n + 1)));
    });
  }
  return {
    followCheckOutput: async (jobId, kept) => {
      stop();
      const log = kept === null ? undefined : JOB_LOGS[kept];
      if (jobId === null || kept === null || log === undefined) {
        fleet.publish({ followed: { state: "none" } });
        return;
      }
      const path = Object.values(LIVE_LOGS).find((one) => name(one) === kept) ?? kept;
      const check = kept.replace(/^implement\.1\.live\./, "").replace(/\.log$/, "");
      play(log, (lines, ended) => {
        const followed: FollowedLog = {
          state: "following",
          jobId,
          kept,
          name: check,
          attempt: 1,
          path,
          fromLine: 1,
          lines,
          ...(ended === undefined ? {} : { ended }),
        };
        fleet.publish({ followed });
      });
    },
    followLandCheck: async (at: LandCheckAt | null) => {
      stop();
      const log = at === null ? undefined : LAND_LOGS[`${at.branch} ${at.check}`];
      if (at === null || log === undefined) {
        fleet.publish({ landFollowed: { state: "none" } });
        return;
      }
      play(log, (lines, ended) => {
        const landFollowed: FollowedLandLog = {
          state: "following",
          root: at.root,
          branch: at.branch,
          check: at.check,
          fromLine: 1,
          lines,
          ...(ended === undefined ? {} : { ended }),
        };
        fleet.publish({ landFollowed });
      });
    },
  };
}

function name(path: string): string {
  return path.slice(path.lastIndexOf("/") + 1);
}
