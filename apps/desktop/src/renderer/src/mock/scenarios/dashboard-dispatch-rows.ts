// The Dashboard with nothing waiting on the owner and a good deal under way: two Jobs at work, one
// queued, two Sessions (one mid-turn with a long thread, one idle), and in Done a Job that
// finished, one that failed, a Session that ended, and the merge line's landed branches, which Done
// does not list. The walk `dashboard-dispatch-rows` plays it.

import { repository } from "@armada/screens/src/fixtures/build/base";
import { mergeLines } from "@armada/screens/src/fixtures/build/merge-line";
import type { Session, SessionRow } from "@armada/screens/src/draft/sessions";
import type { CallView } from "@armada/jobs/draft/calls";
import { completedFailed, completedSuccess, queued } from "@armada/jobs/fixtures/build/index";
import { featureRunning } from "@armada/jobs/fake";
import type { JobFixture } from "@armada/screens/src/fixtures/fixture";

import { asRow, holding } from "../holding";
import type { Scenario } from "../moment";
import { sessionsStore } from "../sessions/script";

const OWNER = repository().manifest!.id;

/** A Job on the picked repository, not yet cleared, so it stays on the Dashboard. */
function kept(fixture: JobFixture): JobFixture {
  const { reclaimed_at: _cleared, ...job } = { ...fixture.job, owner_manifest_id: OWNER };
  return { ...fixture, job };
}

const pin = kept(asRow(featureRunning(), 81, "dash-pin", "Pin the store clock"));
const fold = kept(asRow(featureRunning(), 82, "dash-fold", "Fold the two notification routes into one and drop the retired route with the tests that covered it"));
const retire = kept(asRow(queued(), 83, "dash-retire", "Retire the sleep calls in the store tests"));
const shipped = kept(asRow(completedSuccess(), 84, "dash-shipped", "Add the heartbeat clock to the writer"));
const failed = kept(asRow(completedFailed(), 85, "dash-failed", "Shorten the reconnect wait"));

const now: Record<string, CallView> = {
  [pin.job.id]: { running: [{ key: "r", of: "drone", name: "Drone on Implement", line: "Replacing Instant::now in store/clock.rs", state: "running" }] },
  [fold.job.id]: { running: [{ key: "r", of: "drone", name: "Drone on Implement", line: "Moving the retired route's tests into notify/tests.rs and deleting the old module once nothing imports it", state: "running" }] },
};

const ago = (minutes: number) => new Date(Date.now() - minutes * 60_000).toISOString();
const quiet = { state: "idle" } as const;

const message = (id: string, from: "agent" | "you", text: string): SessionRow => ({ id, at: "14:00:00", kind: "message", from: { kind: from }, text });
const tool = (id: string, text: string): SessionRow => ({ id, at: "14:00:00", kind: "tool", text });

/** A thread longer than its pane, so the newest message is below the fold until the pane opens at it. */
const THREAD: SessionRow[] = [
  message("m0", "you", "List the migrations since 0042 and say which touch the clock column."),
  tool("m1", "Read crates/store/migrations/0043_rename_clock.sql"),
  tool("m2", "Read crates/store/migrations/0044_add_heartbeat.sql"),
  message("m3", "agent", "Two migrations since 0042: 0043 renames the clock column and 0044 adds the heartbeat table."),
  tool("m4", "Read crates/store/src/schema.rs"),
  tool("m5", "grep -rn clock_at crates/store"),
  message("m6", "agent", "Nine call sites still read clock_at, and 0043 renamed it to written_at."),
  tool("m7", "Read crates/store/src/clock.rs"),
  message("m8", "you", "Which of the nine are in tests?"),
  tool("m9", "grep -rn clock_at crates/store/tests"),
  message("m10", "agent", "Four of the nine, all in crates/store/tests/flaky.rs."),
  tool("m11", "Edit crates/store/tests/flaky.rs"),
  tool("m12", "Edit crates/store/src/clock.rs"),
  message("m13", "agent", "Renamed the four test reads to written_at. Checking the other five call sites."),
  tool("m14", "Read crates/store/src/write.rs"),
  tool("m15", "Read crates/store/src/open.rs"),
  message("m16", "agent", "Reading the migrations against the store schema."),
];

const SESSIONS: Session[] = [
  { id: "s20", title: "Migration notes", turn: { state: "working" }, lastTurn: "14:05", lastTurnAt: ago(1), rows: THREAD, attachments: [] },
  {
    id: "s21",
    title: "Release script",
    turn: quiet,
    lastTurn: "13:40",
    lastTurnAt: ago(25),
    rows: [message("r0", "agent", "The release script now tags from the merge commit rather than the branch tip, and it refuses to run while main is red or a pull request is still queued behind it.")],
    attachments: [],
  },
  { id: "s22", title: "Flaky store test", dead: "ended", turn: quiet, lastTurn: "11:40", lastTurnAt: ago(150), rows: [message("e0", "agent", "Done.")], attachments: [] },
];

function build(): Scenario {
  const base = holding("dashboard-dispatch-rows", "The Dashboard under way: Jobs at work, Sessions mid-thread, and what is over", [pin, fold, retire, shipped, failed]);
  const none = { id: "x", number: 0, title: "", branch: "", slot: 0 };
  return {
    ...base,
    state: { ...base.state, repository: repository().root, mergeLines: { lines: [mergeLines().lines[0]!] } },
    draft: {
      calls: now,
      sessions: (board) =>
        sessionsStore([none, none], none, [], board, [], SESSIONS, [
          ["s20", "0043 also drops the index on clock_at, so open.rs needs the new one before the first write."],
          ["s20", "Adding the index to open.rs and running the store tests."],
        ]),
    },
  };
}

export const s205DashboardDispatchRows: Scenario = build();
