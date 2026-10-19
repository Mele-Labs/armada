// A done Session with something still waiting on the person, a pull request, four branches in four slots, a slot no
// branch is on and three artifacts, over the `sessions` scenario: every kind the ledger's bar narrows to, and the waiting item in the thread.
// The walk `ledger-filter-bar` plays it.

import type { Session } from "@armada/screens/src/draft/sessions";

import type { Scenario } from "../moment";
import { s200Sessions } from "./sessions";

const minutesAgo = (minutes: number) => new Date(Date.now() - minutes * 60_000).toISOString();

const holding: Session = {
  id: "s72",
  terminal: true,
  title: "Lock the store migrations",
  turn: { state: "idle" },
  lastTurnAt: minutesAgo(4),
  rows: [],
  done: { since: minutesAgo(1) },
  attachments: [
    { kind: "slot", slot: 9 },
    { kind: "branch", name: "store/migration-lock", slot: 3 },
    { kind: "branch", name: "store/backfill-orders", slot: 5 },
    { kind: "branch", name: "store/drop-legacy-index", slot: 8 },
    { kind: "branch", name: "store/migration-docs", slot: 11 },
    {
      kind: "pull_request",
      number: 1862,
      title: "Lock the store migrations",
      branch: "store/migration-lock",
      address: "https://example.com/pull/1862",
      checks: { state: "passed" },
      state: "open",
      auto: false,
    },
    { kind: "artifact", form: "doc", id: "https://example.com/artifact/migration-notes", title: "Migration timing notes" },
    { kind: "artifact", form: "file", id: "/tmp/backfill-dry-run.txt", title: "Backfill dry run" },
    { kind: "artifact", form: "page", id: "https://example.com/artifact/lock-contention", title: "Lock contention" },
  ],
  waitingFor: [{ id: "a1", text: "Approve #1862", since: minutesAgo(3), source: "agent", act: { kind: "approve_pr", target: "1862" } }],
};

export const s265LedgerFilterBar: Scenario = {
  ...s200Sessions,
  name: "ledger-filter-bar",
  says: "A Session holding an item waiting on you, a pull request, four branches in four slots, a bare slot and three artifacts",
  draft: { sessions: (board) => s200Sessions.draft!.sessions!(board, [holding]) },
};
