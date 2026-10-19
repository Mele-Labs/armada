// A Session that hands the owner a restart onto the preview, beside an ordinary command to run. The
// restart reads as its label and is one key on the Session's own list, as it is on its call card; the
// command keeps its block with Run. Over the `sessions` scenario. The walk `refresh-preview-on-sessions` plays it.

import type { Session } from "@armada/screens/src/draft/sessions";

import type { Scenario } from "../moment";
import { s200Sessions } from "./sessions";

const minutesAgo = (minutes: number) => new Date(Date.now() - minutes * 60_000).toISOString();

const RESTART = "scripts/preview --restart";
const TEST = "pnpm test";

const waiting: Session = {
  id: "s17",
  title: "Pin the store clock",
  turn: { state: "idle" },
  lastTurnAt: minutesAgo(2),
  rows: [
    { id: "p1", at: "14:36:10", kind: "message", from: { kind: "you" }, text: "Pin the clock in the store tests." },
    { id: "p2", at: "14:40:11", kind: "message", from: { kind: "agent" }, text: "The change is on the branch. Both of these are yours to run." },
  ],
  attachments: [{ kind: "slot", slot: 3 }, { kind: "branch", name: "fix/pin-store-clock", slot: 3 }],
  waitingFor: [
    { id: "p-preview", text: "Restart onto the preview", since: minutesAgo(3), source: "agent", act: { kind: "run", target: RESTART } },
    { id: "p-test", text: "Run the tests", since: minutesAgo(2), source: "agent", act: { kind: "run", target: TEST } },
  ],
};

export const s241RefreshPreviewOnSessions: Scenario = {
  ...s200Sessions,
  name: "refresh-preview-on-sessions",
  says: "A Session waiting on a restart onto the preview and on a command, on the Session's list and on its call card",
  draft: { sessions: (board) => s200Sessions.draft!.sessions!(board, [waiting]) },
};
