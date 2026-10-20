// A Session that stopped on a question in words, with no options and nothing to run, open, or approve.
// Its call in the Cockpit ends in the Session's own message box. Over the `sessions` scenario. The walk
// `sessionCallComposer` plays it.

import type { Session } from "@armada/screens/src/draft/sessions";

import type { Scenario } from "../moment";
import { s200Sessions } from "./sessions";

const minutesAgo = (minutes: number) => new Date(Date.now() - minutes * 60_000).toISOString();

const asking: Session = {
  id: "s31",
  title: "Cap the retry ceiling",
  turn: { state: "idle" },
  lastTurnAt: minutesAgo(2),
  rows: [
    { id: "q1", at: "14:44:03", kind: "message", from: { kind: "you" }, text: "Cap the retry backoff in the store client." },
    { id: "q2", at: "14:51:20", kind: "message", from: { kind: "agent" }, text: "The backoff doubles with no ceiling. I can cap it where the client is built, or in the retry loop." },
    { id: "q3", at: "14:52:09", kind: "message", from: { kind: "agent" }, text: "Where should the cap live, and at what value?" },
  ],
  attachments: [],
  waitingFor: [{ id: "w1", text: "Where should the cap live, and at what value?", since: minutesAgo(2), source: "agent" }],
};

export const s210SessionCallComposer: Scenario = {
  ...s200Sessions,
  name: "session-call-composer",
  says: "A Session's question in words, as a call in the Cockpit that ends in the Session's own box",
  draft: { sessions: (board) => s200Sessions.draft!.sessions!(board, [asking]) },
};
