// A Session that landed no pull request but made a page and a Doc, and says it has nothing left: the
// callout lists those. A picture it looked at is in its Artifacts and not in the callout. Over the `sessions`
// scenario. The walk `session-all-done-artifacts` plays it.

import type { Session } from "@armada/screens/src/draft/sessions";

import type { Scenario } from "../moment";
import { s200Sessions } from "./sessions";

const minutesAgo = (minutes: number) => new Date(Date.now() - minutes * 60_000).toISOString();

const finished: Session = {
  id: "s15",
  title: "Write up the store clock",
  turn: { state: "idle" },
  lastTurnAt: minutesAgo(2),
  work: { workflow: { name: "Feature", steps: ["Plan the change", "Implement", "Review the change"], at: 2 } },
  rows: [
    { id: "a1", at: "14:38:02", kind: "message", from: { kind: "you" }, text: "Write up what the store clock does." },
    { id: "a2", at: "14:40:11", kind: "message", from: { kind: "agent" }, text: "The write-up is published." },
  ],
  attachments: [
    { kind: "artifact", form: "page", id: "https://example.com/artifact/clock-findings", title: "Store clock findings" },
    { kind: "artifact", form: "doc", id: "https://example.com/artifact/clock-writeup", title: "Store clock write-up" },
    { kind: "artifact", form: "image", id: "/tmp/ledger-screenshot.png", title: "ledger-screenshot.png" },
  ],
  done: { since: minutesAgo(2) },
};

export const s208SessionAllDoneArtifacts: Scenario = {
  ...s200Sessions,
  name: "session-all-done-artifacts",
  says: "A Session that made a page and a Doc and says it has nothing left to do",
  draft: { sessions: (board) => s200Sessions.draft!.sessions!(board, [finished]) },
};
