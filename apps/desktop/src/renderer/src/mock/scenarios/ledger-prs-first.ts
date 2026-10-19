// A Session that says it is done and still has an item waiting on the person, which stays in the thread; it holds a pull request and three branches, over the `sessions`
// scenario. The ledger draws Pull requests, then Branches. The walk
// `ledger-prs-first` plays it.

import type { Session } from "@armada/screens/src/draft/sessions";

import type { Scenario } from "../moment";
import { s200Sessions } from "./sessions";

const minutesAgo = (minutes: number) => new Date(Date.now() - minutes * 60_000).toISOString();

const holding: Session = {
  id: "s71",
  terminal: true,
  title: "Lock the store migrations",
  turn: { state: "idle" },
  lastTurnAt: minutesAgo(4),
  rows: [],
  done: { since: minutesAgo(1) },
  attachments: [
    { kind: "branch", name: "store/migration-lock", slot: 3 },
    { kind: "branch", name: "store/backfill-orders", slot: 5 },
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
  ],
  waitingFor: [{ id: "a1", text: "Approve #1862", since: minutesAgo(3), source: "agent", act: { kind: "approve_pr", target: "1862" } }],
};

export const s264LedgerPrsFirst: Scenario = {
  ...s200Sessions,
  name: "ledger-prs-first",
  says: "A Session waiting on a pull request approval, holding that pull request and three branches",
  draft: { sessions: (board) => s200Sessions.draft!.sessions!(board, [holding]) },
};
