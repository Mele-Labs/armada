// A Session that holds five branches, each in its own slot, and one slot with no branch on it, over the
// `sessions` scenario. The ledger has no Worktree Slot section: each branch ends in its slot's chip, and
// the bare slot keeps a row under Branches. The walk `ledger-slot-chips` plays it.

import type { Session } from "@armada/screens/src/draft/sessions";

import type { Scenario } from "../moment";
import { s200Sessions } from "./sessions";

const minutesAgo = (minutes: number) => new Date(Date.now() - minutes * 60_000).toISOString();

const holding: Session = {
  id: "s70",
  terminal: true,
  title: "Split the store migrations",
  turn: { state: "idle" },
  lastTurnAt: minutesAgo(4),
  rows: [],
  attachments: [
    { kind: "slot", slot: 2 },
    { kind: "slot", slot: 3 },
    { kind: "slot", slot: 5 },
    { kind: "slot", slot: 8 },
    { kind: "slot", slot: 11 },
    { kind: "slot", slot: 9 },
    { kind: "branch", name: "store/split-migrations", slot: 2 },
    { kind: "branch", name: "store/migration-lock", slot: 3 },
    { kind: "branch", name: "store/backfill-orders", slot: 5 },
    { kind: "branch", name: "store/drop-legacy-index", slot: 8 },
    { kind: "branch", name: "store/migration-docs", slot: 11 },
  ],
};

export const s263LedgerSlotChips: Scenario = {
  ...s200Sessions,
  name: "ledger-slot-chips",
  says: "A Session holding five branches in five slots and a slot with no branch",
  draft: { sessions: (board) => s200Sessions.draft!.sessions!(board, [holding]) },
};
