// A note pinned on the Usage mock, landed in the Job that showed it: the thread holds his words alone, and
// the agent's reply names the line Fleet found. Over the `sessions` scenario. The walk `pinnedNoteLands` plays it.

import type { Session } from "@armada/screens/src/draft/sessions";

import type { Scenario } from "../moment";
import { s200Sessions } from "./sessions";

const minutesAgo = (minutes: number) => new Date(Date.now() - minutes * 60_000).toISOString();

const REPLY = [
  "Opened `packages/components/src/compositions/UsagePage/UsagePage.tsx:171`, the row you pinned.",
  "",
  "Making its bar darker.",
].join("\n");

const landed: Session[] = [
  {
    id: "s14",
    title: "Usage page",
    turn: { state: "working" },
    standing: "working",
    lastTurnAt: minutesAgo(1),
    rows: [
      { id: "r1", at: "14:41:50", kind: "message", from: { kind: "you" }, text: "this bar is too faint" },
      { id: "r2", at: "14:42:04", kind: "message", from: { kind: "agent" }, text: REPLY },
    ],
    attachments: [],
  },
];

export const s282PinnedNoteLands: Scenario = {
  ...s200Sessions,
  name: "pinned-note-lands",
  says: "A Job holding a note pinned on its page: his words, and the agent's reply naming the line",
  draft: { sessions: (board) => s200Sessions.draft!.sessions!(board, landed) },
};
