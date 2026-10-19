// The Cockpit after a night of Sleep mode: Sleep still on with the night's decisions made
// (`mock/sleep.ts`), which arrive as calls once the first press turns the moon off. Both were made in
// Sessions, which are here to be opened. The walk `sleepCockpit` plays it.

import { repository } from "@armada/screens/src/fixtures/repository";

import type { Session } from "@armada/screens/src/draft/sessions";

import { connected } from "../moment";
import type { Scenario } from "../moment";
import { BRANCH_SESSION, LUNCH_SESSION } from "../sleep";
import { s200Sessions } from "./sessions";

const minutesAgo = (minutes: number) => new Date(Date.now() - minutes * 60_000).toISOString();

/** A Session that stopped on one question, the one Sleep answered. */
const asked = (id: string, title: string, request: string, question: string, at: number): Session => ({
  id,
  title,
  turn: { state: "idle" },
  lastTurnAt: minutesAgo(at),
  rows: [
    { id: `${id}-1`, at: "02:04:10", kind: "message", from: { kind: "you" }, text: request },
    { id: `${id}-2`, at: "02:10:31", kind: "message", from: { kind: "agent" }, text: question },
  ],
  attachments: [],
});

const lunch = asked(LUNCH_SESSION, "Order the lunch", "Order the lunch for Friday.", "Which size?", 420);
const naming = asked(BRANCH_SESSION, "Name the branch", "Name the branch for the reconnect change.", "fix or feat?", 380);

function build(): Scenario {
  const sessions = s200Sessions.draft!.sessions!;
  return {
    name: "sleep-cockpit",
    says: "The Cockpit with a night of Sleep decisions to review",
    state: { ...connected([], [repository()]), repository: repository().root },
    draft: { sessions: (board) => sessions(board, [lunch, naming]) },
  };
}

export const s209SleepCockpit: Scenario = build();
