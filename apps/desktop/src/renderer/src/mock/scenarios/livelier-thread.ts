// A Job's thread made roomier and livelier, for the walk `livelier-thread`: the agent's messages and his, a run that ended with
// its last lines under it, one still running with its time ticking, and the agent's last message a question, so the quick bar
// offers its answers. The window holds a thread look (`thread-look-here.tsx`), so the Aa panel, the quick bar and Talk are drawn.
// A walk's first `later` ends the running run; its second brings a Job just dispatched into the Cockpit's list. Over `sessions`.

import type { Session, SessionRow } from "@armada/screens/src/draft/sessions";

import type { Scenario } from "../moment";
import type { SessionsStore } from "../sessions/script";
import { s200Sessions } from "./sessions";

const JOB = "s40";
const RUN = "lt-run-suite";

const ago = (seconds: number) => new Date(Date.now() - seconds * 1000).toISOString();

const SUITE_TAIL = ["running 212 tests", "test ledger::clock_is_pinned ... ok", "test ledger::reads_after_write ... ok", "test slots::lease_is_one_line ... ok", "test result: ok. 212 passed; 0 failed; finished in 41.06s"].join("\n");

const thread = (): SessionRow[] => [
  { id: "lt1", at: ago(540), kind: "message", from: { kind: "agent" }, text: "The store tests read the clock before the write lands. On a slow runner the ledger is still empty when the test looks." },
  { id: "lt2", at: ago(480), kind: "message", from: { kind: "you" }, text: "Pin the clock in the test module, not in the store." },
  { id: "lt3", at: ago(420), kind: "message", from: { kind: "agent" }, text: "Done. The pin is in `tests/clock.rs` and the store is untouched." },
  {
    id: "lt-run-ledger",
    at: ago(360),
    kind: "ran",
    command: "cargo test -p store ledger",
    code: 0,
    tail: ["running 14 tests", "test ledger::writes_then_reads ... ok", "test ledger::clock_is_pinned ... ok", "test result: ok. 14 passed; 0 failed; finished in 0.41s"].join("\n"),
  },
  { id: "lt4", at: ago(300), kind: "message", from: { kind: "you" }, text: "Run the whole store suite once." },
  { id: RUN, at: ago(38), kind: "ran", command: "cargo test -p store", code: null, tail: "", running: true, since: ago(38) },
  { id: "lt5", at: ago(30), kind: "message", from: { kind: "agent" }, text: "The suite is running. When it passes, should I restart Fleet on the preview so you can try it?" },
];

const job = (): Session => ({
  id: JOB,
  title: "Store clock fix",
  turn: { state: "idle" },
  lastTurnAt: ago(30),
  rows: thread(),
  attachments: [{ kind: "slot", slot: 3 }, { kind: "branch", name: "fix/pin-store-clock", slot: 3 }],
});

/** The Job dispatched while he walks: it lifts off into the list. */
const dispatched = (): Session => ({
  id: "s41",
  title: "Quiet the slot logs",
  turn: { state: "working" },
  lastTurnAt: new Date().toISOString(),
  rows: [{ id: "q1", at: new Date().toISOString(), kind: "message", from: { kind: "you" }, text: "One line per lease in the slot log, not three." }],
  attachments: [],
  work: {},
});

/**
 * The Sessions store, with the walk's two moments in front of the usual ones: the run ends, then a Job arrives. **The new Job is
 * drawn beside the store's own**, which never holds it, so it can be looked at and opened but not spoken to.
 */
function livelier(store: SessionsStore): SessionsStore {
  const late = dispatched();
  let shown = false;
  let step = 0;
  let base: readonly Session[] | undefined;
  let drawn: readonly Session[] = [];
  const listeners = new Set<() => void>();
  const withLate = (now: readonly Session[]): readonly Session[] => {
    if (!shown) return now;
    if (now !== base) {
      base = now;
      drawn = [...now, late];
    }
    return drawn;
  };
  const endRun = () =>
    store.edit(JOB, (one) => ({
      ...one,
      rows: one.rows.map((row): SessionRow => (row.id === RUN && row.kind === "ran" ? { id: row.id, at: row.at, kind: "ran", command: row.command, code: 0, tail: SUITE_TAIL } : row)),
    }));
  return Object.assign(store, {
    get: ((get) => () => withLate(get()))(store.get),
    subscribe: ((subscribe) => (on: () => void) => {
      listeners.add(on);
      const off = subscribe(on);
      return () => {
        listeners.delete(on);
        off();
      };
    })(store.subscribe),
    later: ((later) => () => {
      step += 1;
      if (step === 1) return endRun();
      if (step === 2) {
        shown = true;
        listeners.forEach((on) => on());
        return;
      }
      later();
    })(store.later),
  });
}

export const s341LivelierThread: Scenario = {
  ...s200Sessions,
  name: "livelier-thread",
  says: "A Job's thread with room between messages, his words as a bubble, a run ticking and one ended, and the quick bar over the box",
  draft: {
    sessions: (board) => livelier(s200Sessions.draft!.sessions!(board, [job()])),
    threadLook: {
      look: {
        agentName: "",
        showTimes: true,
        showAgentName: true,
        textSize: "base",
        quick: [
          { id: "learnt:done", text: "Just get it done", kind: "learnt" },
          { id: "learnt:walk", text: "Walk me through it", kind: "learnt" },
          { id: "saved:land", text: "Land it", kind: "saved" },
        ],
        learn: true,
        voice: "apple",
      },
      asked: { [JOB]: ["Restart it", "Not yet"] },
      heard: "Hold the restart until the suite is green, then show me the walk",
    },
  },
};
