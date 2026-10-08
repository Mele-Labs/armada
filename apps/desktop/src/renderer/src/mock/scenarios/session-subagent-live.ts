// A Session with a subagent still running and one that has finished. Pressing a Subagents row opens
// that subagent's own thread beside the Session: a running one gains rows as it is read again, and
// both end on the report. Over the `sessions` scenario with one more Session. The walk
// `session-subagent-live` plays it.

import type { Session } from "@armada/screens/src/draft/sessions";

import type { Scenario } from "../moment";
import { s200Sessions } from "./sessions";

const more: Session[] = [
  {
    id: "s13",
    terminal: true,
    title: "Read the CI history",
    turn: { state: "idle" },
    lastTurn: "14:40",
    rows: [],
    attachments: [
      { kind: "branch", name: "fix/ci-history", slot: 2 },
      { kind: "subagent", id: "live1", task: "Read the CI history of store_flaky", state: "running" },
      { kind: "subagent", id: "done1", task: "Find other tests that read the wall clock", state: "done" },
    ],
  },
];

export const s204SessionSubagentLive: Scenario = {
  ...s200Sessions,
  name: "session-subagent-live",
  says: "A running subagent's own thread, drawn live beside its Session, and a finished one's ending on its report",
  draft: { sessions: (board) => s200Sessions.draft!.sessions!(board, more) },
};
