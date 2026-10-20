// The Dashboard under way, as tiles: Sessions working, idle and asking, and in Done the Sessions that
// are over, with the merge line's landed branches, which Done does not list.

import { repository } from "@armada/screens/src/fixtures/repository";
import { mergeLines } from "@armada/screens/src/fixtures/merge-line";
import type { Session, SessionRow } from "@armada/screens/src/draft/sessions";

import { holding } from "../holding";
import type { Scenario } from "../moment";
import { sessionsStore } from "../sessions/script";

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
  { id: "s20", title: "Migration notes", turn: { state: "working" }, lastTurnAt: ago(1), rows: THREAD, attachments: [] },
  {
    id: "s21",
    title: "Release script",
    turn: quiet,
    lastTurnAt: ago(25),
    rows: [message("r0", "agent", "The release script now tags from the merge commit rather than the branch tip."), tool("r1", "Edit scripts/release"), message("r2", "agent", "It refuses to run while main is red.")],
    attachments: [],
  },
  {
    id: "s23",
    title: "Flaky store test",
    turn: quiet,
    lastTurnAt: ago(3),
    rows: [message("f0", "agent", "The flaky test is the 200ms sleep in store_test.rs."), tool("f1", "Edit crates/store/tests/store_test.rs"), message("f2", "agent", "The branch is ready to push.")],
    attachments: [],
    asked: { command: "git push --force-with-lease origin fix/flaky-store" },
  },
  { id: "s24", title: "Lint sweep", turn: { state: "working" }, lastTurnAt: ago(2), rows: [tool("l0", "cargo clippy --workspace"), tool("l1", "Edit crates/fleet/src/lib.rs"), tool("l2", "cargo clippy --workspace"), message("l3", "agent", "Eleven warnings left."), tool("l4", "Edit crates/store/src/open.rs")], attachments: [] },
  { id: "s22", title: "Migration spike", dead: "ended", turn: quiet, lastTurnAt: ago(150), rows: [message("e0", "agent", "Done."), tool("e1", "Read crates/store/src/open.rs"), message("e2", "agent", "Named migrations apply in order.")], attachments: [] },
  { id: "s25", title: "Release notes", dead: "ended", turn: quiet, lastTurnAt: ago(300), rows: [message("n0", "agent", "Done.")], attachments: [] },
];

function build(): Scenario {
  const base = holding("dashboard-dispatch-rows", "The Dashboard as tiles: Sessions under way, and what is over");
  return {
    ...base,
    state: { ...base.state, repository: repository().root, mergeLines: { lines: [mergeLines().lines[0]!] } },
    draft: {
      sessions: () =>
        sessionsStore(SESSIONS, [
          ["s20", "0043 also drops the index on clock_at, so open.rs needs the new one before the first write."],
          ["s20", "Adding the index to open.rs and running the store tests."],
        ]),
    },
  };
}

export const s205DashboardDispatchRows: Scenario = build();
