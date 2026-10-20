// A Session that waits on the person for something it gave no act: the ledger draws it with a Reply
// glyph, and a press opens a box under the row. Over the `sessions` scenario. The walk `session-reply`
// plays it.

import type { Session } from "@armada/screens/src/draft/sessions";

import type { Scenario } from "../moment";
import { s200Sessions } from "./sessions";

const minutesAgo = (minutes: number) => new Date(Date.now() - minutes * 60_000).toISOString();

const waiting: Session = {
  id: "s15",
  title: "Pin the store clock",
  turn: { state: "idle" },
  lastTurnAt: minutesAgo(3),
  rows: [
    { id: "r1", at: "14:38:02", kind: "message", from: { kind: "you" }, text: "Pin the clock in the store tests." },
    { id: "r2", at: "14:40:11", kind: "message", from: { kind: "agent" }, text: "The change is up. I need two answers from you." },
  ],
  attachments: [{ kind: "slot", slot: 3 }, { kind: "branch", name: "fix/pin-store-clock", slot: 3 }],
  waitingFor: [
    { id: "p1", text: "Which date should the clock pin to?", since: minutesAgo(3), source: "agent" },
    { id: "p2", text: "Keep the old fixture or drop it?", since: minutesAgo(2), source: "agent" },
    { id: "p3", text: "Name the helper eventually, or settle?", since: minutesAgo(1), source: "agent", act: { kind: "answer", target: "" } },
  ],
};

export const s209SessionReply: Scenario = {
  ...s200Sessions,
  name: "session-reply",
  says: "A Session waiting on the person for two things it gave no act",
  draft: { sessions: (board) => s200Sessions.draft!.sessions!(board, [waiting]) },
};
