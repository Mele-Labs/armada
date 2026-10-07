// The Sessions list with a name over its ledger, and a Session renamed from its header. Over the
// `sessions` scenario, with three more Sessions open beside its own: one without a title, one from a terminal whose first
// prompt came wrapped in an agent message, and one holding six pull requests. The walk
// `session-names` plays it.

import type { Session, SessionAttachment } from "@armada/screens/src/draft/sessions";

import type { Scenario } from "../moment";
import { s200Sessions } from "./sessions";

const idle = { state: "idle" } as const;
const minutesAgo = (minutes: number) => new Date(Date.now() - minutes * 60_000).toISOString();

const pull = (number: number, checks: "passed" | "pending" | "failed"): SessionAttachment => ({
  kind: "pull_request",
  number,
  title: `Retire sleeps, part ${number - 1850}`,
  branch: `fix/retire-sleeps-${number - 1850}`,
  address: `https://example.com/pull/${number}`,
  checks: checks === "failed" ? { state: "failed", failing: "store: 2 failed" } : { state: checks },
  state: "open",
  auto: false,
});

const more: Session[] = [
  {
    id: "s5",
    terminal: true,
    title: "Review the ledger change",
    turn: idle,
    lastTurn: "14:02",
    lastTurnAt: minutesAgo(31),
    rows: [],
    attachments: [{ kind: "branch", name: "review/ledger-change", slot: 1 }],
  },
  {
    id: "s6",
    terminal: true,
    title: "Retire the sleeps",
    turn: idle,
    lastTurn: "13:15",
    lastTurnAt: minutesAgo(3 * 24 * 60),
    rows: [],
    attachments: [
      { kind: "slot", slot: 2 },
      { kind: "branch", name: "fix/retire-sleeps-1", slot: 2 },
      pull(1851, "passed"),
      pull(1852, "passed"),
      pull(1853, "pending"),
      pull(1854, "failed"),
      pull(1855, "passed"),
      pull(1856, "pending"),
    ],
  },
  {
    id: "s7",
    terminal: true,
    turn: idle,
    lastTurn: "12:40",
    lastTurnAt: minutesAgo(5 * 24 * 60),
    rows: [],
    attachments: [{ kind: "branch", name: "spike/untitled", slot: 3 }],
  },
];

export const s201SessionNames: Scenario = {
  ...s200Sessions,
  name: "session-names",
  says: "The Sessions list with the name over its ledger, and a Session renamed from its header",
  draft: { sessions: (board) => s200Sessions.draft!.sessions!(board, more) },
};
