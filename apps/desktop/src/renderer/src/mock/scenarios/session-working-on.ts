// What each Session is doing, and what the person is waiting on it for. Four Sessions over the
// `sessions` scenario: one working through a todo list, one working with no list, one working that also
// waits on the person, and one that says it is done. `now` is what Fleet detected from the last tool call; the todos are the agent's own report.
// Each `activity` is built from the wire's shape and read by the mapper a real record goes through.
// The walk `session-working-on` plays it.

import type { Session } from "@armada/screens/src/draft/sessions";
import { activityOf } from "@armada/screens/src/sessions-wire";

import type { Scenario } from "../moment";
import { s200Sessions } from "./sessions";

const minutesAgo = (minutes: number) => new Date(Date.now() - minutes * 60_000).toISOString();

const asked = (id: string, text: string) => [{ id, at: "14:00:02", kind: "message", from: { kind: "you" }, text } as const];

const gate: Session = {
  id: "s41",
  title: "Gate policy for every run",
  turn: { state: "working" },
  lastTurnAt: minutesAgo(1),
  rows: [
    ...asked("g1", "Make the check run its policy on every pull request, not only the first."),
    { id: "g2", at: "14:06:10", kind: "tool", text: "Read crates/fleet/src/gate.rs" },
    { id: "g3", at: "14:11:48", kind: "tool", text: "Edit crates/fleet/src/gate.rs" },
  ],
  attachments: [{ kind: "slot", slot: 3 }, { kind: "branch", name: "fleet/gate-policy-every-run", slot: 3 }],
  activity: activityOf({
    now: "Editing gate.rs",
    todos: [
      { text: "Read the gate policy", state: "done" },
      { text: "Run the policy on every pull request", state: "doing" },
      { text: "Cover a second run in the gate tests", state: "todo" },
      { text: "Update the gate contract", state: "todo" },
    ],
  }),
};

const backfill: Session = {
  id: "s42",
  title: "Backfill dry run",
  turn: { state: "working" },
  lastTurnAt: minutesAgo(1),
  rows: [
    ...asked("b1", "Dry-run the backfill against the copy."),
    { id: "b2", at: "14:10:40", kind: "tool", text: "Read crates/store/src/backfill.rs" },
    { id: "b3", at: "14:11:02", kind: "tool", text: "Bash cargo check -p store" },
  ],
  attachments: [],
  activity: activityOf({ now: "Running cargo check" }),
};

const both: Session = {
  id: "s44",
  title: "Pin the store clock",
  turn: { state: "working" },
  lastTurnAt: minutesAgo(2),
  rows: [
    ...asked("p1", "Pin the clock in the store tests."),
    { id: "p2", at: "14:09:30", kind: "tool", text: "Edit crates/store/tests/store_test.rs" },
    { id: "p3", at: "14:10:01", kind: "message", from: { kind: "agent" }, text: "The change is up as #1843. I am still on the second fixture." },
  ],
  attachments: [
    { kind: "slot", slot: 4 },
    { kind: "branch", name: "fix/pin-store-clock", slot: 4 },
    { kind: "pull_request", number: 1843, title: "Pin the store clock", branch: "fix/pin-store-clock", address: "https://example.com/pull/1843", checks: { state: "passed" }, state: "open", auto: false },
  ],
  activity: activityOf({
    now: "Editing store_test.rs",
    todos: [
      { text: "Pin the clock in the store tests", state: "done" },
      { text: "Open the pull request", state: "done" },
      { text: "Replace the second fixture", state: "doing" },
      { text: "Rerun the store tests", state: "todo" },
    ],
  }),
  waitingFor: [
    { id: "a1", text: "Approve #1843", since: minutesAgo(2), source: "agent", act: { kind: "approve_pr", target: "1843" } },
    { id: "a2", text: "Keep the old fixture or drop it?", since: minutesAgo(2), source: "agent" },
  ],
};

const finished: Session = {
  id: "s45",
  title: "Order the store migrations",
  turn: { state: "idle" },
  lastTurnAt: minutesAgo(16),
  rows: [
    ...asked("f1", "Order the store migrations by number."),
    { id: "f2", at: "13:58:11", kind: "message", from: { kind: "agent" }, text: "The migrations apply in order and each is recorded once." },
  ],
  attachments: [{ kind: "pull_request", number: 1839, title: "Order the store migrations", branch: "store/order-migrations", address: "https://example.com/pull/1839", checks: { state: "passed" }, state: "merged", auto: false }],
  activity: activityOf({
    todos: [
      { text: "Number the migrations", state: "done" },
      { text: "Record each once", state: "done" },
    ],
  }),
  done: { since: minutesAgo(15) },
};

export const sessionWorkingOn: Scenario = {
  ...s200Sessions,
  name: "session-working-on",
  says: "Four Sessions: a todo list under way, a turn with no list, one that also waits on the person, and one that is done",
  draft: { sessions: (board) => s200Sessions.draft!.sessions!(board, [gate, backfill, both, finished]) },
};
