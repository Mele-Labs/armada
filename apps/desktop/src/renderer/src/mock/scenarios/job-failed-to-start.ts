// A Job whose agent process never started, even on Fleet's second try, beside Jobs that did. Its row reads a state mark with the cause in its tooltip,
// pulses as a row that needs him does, and its thread ends on a Fleet line with the cause, the agent's own error lines folded, and Start again and
// Report. The walk `job-failed-to-start` plays it.

import type { Session } from "@armada/screens/src/draft/sessions";

import { holding } from "../holding";
import type { Scenario } from "../moment";
import { sessionsStore } from "../sessions/script";

const ago = (minutes: number) => new Date(Date.now() - minutes * 60_000).toISOString();

const working: Session = {
  id: "f1",
  title: "Trim the retry loop",
  turn: { state: "working" },
  standing: "working",
  lastTurnAt: ago(1),
  rows: [
    { id: "f1-u", at: "13:40:00", kind: "message", from: { kind: "you" }, text: "Back off between attempts when the lease is contended." },
    { id: "f1-t", at: "13:41:12", kind: "tool", text: "Read crates/fleet/src/lease.rs" },
  ],
  attachments: [],
};

const replied: Session = {
  id: "f2",
  title: "Pin the store clock",
  turn: { state: "idle" },
  standing: "replied",
  lastTurnAt: ago(10),
  rows: [
    { id: "f2-u", at: "13:30:00", kind: "message", from: { kind: "you" }, text: "Pin the clock in the store tests." },
    { id: "f2-a", at: "13:38:09", kind: "message", from: { kind: "agent" }, text: "The clock is pinned in all four store tests and the fixture is gone." },
  ],
  attachments: [],
};

const failed: Session = {
  id: "f3",
  title: "Rename the slot pool",
  turn: { state: "idle" },
  lastTurnAt: ago(4),
  startFailed: {
    cause: "the agent did not come up within 10 seconds",
    log: ["Error: EACCES: permission denied, open '/Users/devin/.claude.json'", "    at Object.openSync (node:fs:573:18)", "    at readConfig (/usr/local/lib/claude/cli.js:412:9)"],
  },
  rows: [{ id: "f3-u", at: "13:44:30", kind: "message", from: { kind: "you" }, text: "Rename the slot pool to bays in the settings and the docs." }],
  attachments: [],
};

function build(): Scenario {
  const base = holding("job-failed-to-start", "A Job whose agent never started, beside a working Job and a replied one");
  return {
    ...base,
    later: [{}],
    draft: { sessions: () => sessionsStore([working, replied, failed]) },
  };
}

export const jobFailedToStart: Scenario = build();
