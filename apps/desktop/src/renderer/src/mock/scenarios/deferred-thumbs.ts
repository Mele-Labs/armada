// A Job whose Landing holds Deferred: three follow-ups the agent kept and an issue it filed. Each kept row ends in Remind and two thumbs that say whether
// Fleet was right to keep it, and a right press offers the same two; the filed issue has none. Fleet does not take the marks yet. The walk `deferred-thumbs` plays it.

import type { Session } from "@armada/screens/src/draft/sessions";
import { repository } from "@armada/screens/src/fixtures/repository";

import { holding } from "../holding";
import type { Scenario } from "../moment";
import { sessionsStore } from "../sessions/script";

const ago = (minutes: number) => new Date(Date.now() - minutes * 60_000).toISOString();

const SESSION: Session = {
  id: "dt-1",
  title: "Read the Studio registry on start",
  turn: { state: "idle" },
  lastTurnAt: ago(2),
  work: { proposal: { workflow: null } },
  attachments: [{ kind: "issue", number: "2731", title: "Slow fixture load", address: "https://git.example/app/issues/2731", state: "open" }],
  rows: [
    { id: "dt-u", at: "14:02:00", kind: "message", from: { kind: "you" }, text: "Read the Studio registry when Studio starts." },
    { id: "dt-a", at: "14:19:40", kind: "message", from: { kind: "agent" }, text: "The registry is read in Studio start and the Studio tests pass." },
  ],
  later: [
    { id: "later-1", text: "The Studio icon tooltip is still owed.", at: ago(5) },
    { id: "later-2", text: "Rename the registry loader once the Studio page lands.", at: ago(4) },
    { id: "later-3", text: "The empty Studio state needs a second look.", at: ago(3) },
  ],
};

function build(): Scenario {
  const base = holding("deferred-thumbs", "A Job whose Landing holds Deferred: three kept follow-ups with Remind and thumbs, and one filed issue without");
  return {
    ...base,
    state: { ...base.state, repository: repository().root },
    draft: { sessions: () => sessionsStore([SESSION]) },
  };
}

export const deferredThumbs: Scenario = build();
