// A Session that waits on the owner to run two commands, one card each in the Cockpit: one that exits
// clean and one that fails. The card runs the command where the Session works, as its ledger does.
// The walk `callCardRunsCommand` plays it.

import { repository } from "@armada/screens/src/fixtures/repository";
import type { Session } from "@armada/screens/src/draft/sessions";

import { holding } from "../holding";
import type { Scenario } from "../moment";
import { sessionsStore } from "../sessions/script";

const ago = (minutes: number) => new Date(Date.now() - minutes * 60_000).toISOString();

const SESSION: Session = {
  id: "s21",
  title: "Debug Bridge on Merge line",
  turn: { state: "idle" },
  lastTurnAt: ago(2),
  rows: [
    { id: "c1", at: "22:12:40", kind: "message", from: { kind: "you" }, text: "Clean up after the merge line fix." },
    { id: "c2", at: "22:20:43", kind: "message", from: { kind: "agent" }, text: "The slot guard won't let me stop the mock server, and the components tests need a run." },
  ],
  attachments: [],
  waitingFor: [
    { id: "c-stop", text: "Stop the mock server on 5788", since: ago(4), source: "agent", act: { kind: "run", target: "kill $(lsof -ti tcp:5788)" } },
    { id: "c-tests", text: "Run the components tests", since: ago(3), source: "agent", act: { kind: "run", target: "pnpm vitest run packages/components" } },
    { id: "c-preview", text: "Restart onto the preview", since: ago(2), source: "agent", act: { kind: "run", target: "scripts/preview --restart" } },
  ],
};

function build(): Scenario {
  const base = holding("call-card-runs-command", "The Cockpit with one Session waiting on two commands to run, one that exits clean and one that fails");
  return {
    ...base,
    state: { ...base.state, repository: repository().root },
    draft: { sessions: () => sessionsStore([SESSION]) },
  };
}

export const s235CallCardRunsCommand: Scenario = build();
