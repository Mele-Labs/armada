// An Incoming call opens what it is about. Three Sessions ask, in the wire's own `waiting_for`: one with two questions
// (a Session has one `call` on its subject, so only one of its calls is that key), one with a single question, and Helm's
// own Session, which the deck's list does not draw. The claims are in `cockpit-call-owner.test.tsx`.

import type { Session } from "@armada/screens/src/draft/sessions";
import { repository } from "@armada/screens/src/fixtures/repository";

import { holding } from "../holding";
import type { Scenario } from "../moment";
import { sessionsStore } from "../sessions/script";

const OWNER = repository().manifest!.id;
const ago = (minutes: number) => new Date(Date.now() - minutes * 60_000).toISOString();

const asking = (id: string, title: string, items: Session["waitingFor"]): Session => ({
  id,
  title,
  manifestId: OWNER,
  turn: { state: "idle" },
  lastTurnAt: ago(2),
  rows: [{ id: `${id}-a`, at: "14:00:11", kind: "message", from: { kind: "agent" }, text: "A question before I go on." }],
  attachments: [],
  ...(items === undefined ? {} : { waitingFor: items }),
});

const two = asking("d5da70fd-70f9-4394-85c6-26076c9aa8c8", "Session / Job Progress", [
  { id: "ask:progress:0", text: "Which ring should a build draw?", since: ago(3), source: "ask_card", options: [{ label: "One ring" }, { label: "A ring a step" }, { label: "None" }] },
  { id: "ask:progress:1", text: "Where does the ring sit?", since: ago(2), source: "ask_card", options: [{ label: "On the drone" }, { label: "On the bay" }] },
]);
const helm: Session = {
  ...asking("h1", "Helm · armada", [{ id: "ask:helm:0", text: "Which repository should I read first?", since: ago(1), source: "ask_card", options: [{ label: "armada" }, { label: "pocket" }] }]),
  helm: true,
};

function build(): Scenario {
  const base = holding("cockpit-call-owner", "The flight deck with a Session that asks two questions and Helm's own Session asking one");
  return {
    ...base,
    state: { ...base.state, repository: null },
    draft: { sessions: () => sessionsStore([two, helm]) },
  };
}

export const s253CockpitCallOwner: Scenario = build();
