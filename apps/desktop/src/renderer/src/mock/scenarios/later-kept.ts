// What a Job meets when it says it is done with follow-ups still open, shown on fixtures. One Job whose thread holds its done, Fleet's refusal
// naming what is open under Later, and the agent's answer that settles one. The one left is in Progress under Later, with a Remind and a Drop.
// Fleet refuses all_done this way, but to the agent only: the notice in the thread is the mock's. The walk `later-kept` plays it.

import type { Session } from "@armada/screens/src/draft/sessions";
import { repository } from "@armada/screens/src/fixtures/repository";

import { holding } from "../holding";
import type { Scenario } from "../moment";
import { sessionsStore } from "../sessions/script";

const ago = (minutes: number) => new Date(Date.now() - minutes * 60_000).toISOString();

const FILED = "I will file the slow fixture load once this lands.";
const OWED = "The Studio icon tooltip is still owed.";

const SESSION: Session = {
  id: "lk-1",
  title: "Read the Studio registry on start",
  turn: { state: "idle" },
  lastTurnAt: ago(2),
  work: { proposal: { workflow: null } },
  attachments: [],
  rows: [
    { id: "lk-u", at: "14:02:00", kind: "message", from: { kind: "you" }, text: "Read the Studio registry when Studio starts." },
    { id: "lk-a", at: "14:19:40", kind: "message", from: { kind: "agent" }, text: `The registry is read in Studio start and the Studio tests pass. This is done.\n\n${FILED}\n${OWED}` },
    {
      id: "lk-f",
      at: "14:19:41",
      kind: "message",
      from: { kind: "session", id: "fleet", title: "Fleet" },
      text: `all_done refused. Still open under Later:\n\n- ${FILED}\n- ${OWED}\n\nDo each, file it, or drop it with a reason.`,
    },
    { id: "lk-b", at: "14:21:10", kind: "message", from: { kind: "agent" }, text: "Filed #2731 for the slow fixture load." },
  ],
  later: [{ id: "later-2", text: OWED, at: ago(2) }],
};

function build(): Scenario {
  const base = holding("later-kept", "A Job told all_done is refused while follow-ups are open: Fleet's notice in the thread, one follow-up settled, and the one left under Later with Remind and Drop");
  return {
    ...base,
    state: { ...base.state, repository: repository().root },
    draft: { sessions: () => sessionsStore([SESSION]) },
  };
}

export const laterKept: Scenario = build();
