// A Session's thread names two other Sessions by their address, once in backticks (how an agent writes
// one) and once in plain text. Each is a link that opens that Session, the ended one too. Over the
// `sessions` scenario. The walk `session-id-links` plays it.

import type { Session } from "@armada/screens/src/draft/sessions";

import type { Scenario } from "../moment";
import { s200Sessions } from "./sessions";

const now = new Date().toISOString();

const quiet = { turn: { state: "idle" }, lastTurn: "14:40", lastTurnAt: now, attachments: [] } as const;

const reader: Session = {
  ...quiet,
  id: "e165b4dfc0a1",
  title: "Settings loader notes",
  rows: [{ id: "l1", at: "14:40:11", kind: "message", from: { kind: "agent" }, text: "Picking this up from `s-e165b4df`." }],
};

const ended: Session = {
  ...quiet,
  id: "b7d20c9e11aa",
  title: "Retry loop findings",
  dead: "ended",
  rows: [{ id: "e1", at: "13:02:00", kind: "message", from: { kind: "agent" }, text: "Nothing more to add." }],
};

const holder: Session = {
  ...quiet,
  id: "4a9c0d13f7e2",
  title: "Hand-off thread",
  rows: [
    { id: "h1", at: "14:38:02", kind: "message", from: { kind: "you" }, text: "Who has the loader?" },
    { id: "h2", at: "14:40:11", kind: "message", from: { kind: "agent" }, text: "Ask `s-e165b4df`, or read s-b7d20c9e before that. A `s-e165b4df --fix` stays code." },
  ],
};

export const s208SessionIdLinks: Scenario = {
  ...s200Sessions,
  name: "session-id-links",
  says: "A Session's thread names other Sessions by their address",
  draft: { sessions: (board) => s200Sessions.draft!.sessions!(board, [reader, ended, holder]) },
};
