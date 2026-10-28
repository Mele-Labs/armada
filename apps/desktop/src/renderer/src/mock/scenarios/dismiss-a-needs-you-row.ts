// A Session that waits on the owner for a page to look at and a pull request it said it would not merge. Each row of
// its Needs you panel is put away in one press. The walk `dismissARow` plays it.

import { repository } from "@armada/screens/src/fixtures/repository";
import type { Session } from "@armada/screens/src/draft/sessions";

import { holding } from "../holding";
import type { Scenario } from "../moment";
import { sessionsStore } from "../sessions/script";

const ago = (minutes: number) => new Date(Date.now() - minutes * 60_000).toISOString();

const SESSION: Session = {
  id: "s22",
  title: "Docs for the pairing screen",
  turn: { state: "idle" },
  lastTurnAt: ago(2),
  rows: [
    { id: "d1", at: "12:10:40", kind: "message", from: { kind: "you" }, text: "Write the caption for the pairing screen." },
    { id: "d2", at: "12:20:43", kind: "message", from: { kind: "agent" }, text: "The page is up. I won't merge or enable auto-merge on the pull request." },
  ],
  attachments: [],
  waitingFor: [
    { id: "walk:https://git.example/pairing", text: "The pairing screen is ready to look at", since: ago(34), source: "walk", act: { kind: "walk", target: "https://git.example/pairing" } },
    { id: "replied:pairing-pr", text: "I won't merge or enable auto-merge on #1822.", since: ago(20), source: "replied", act: { kind: "approve_pr", target: "#1822" } },
  ],
};

function build(): Scenario {
  const base = holding("dismiss-a-needs-you-row", "The Cockpit with one Session waiting on a walk and a pull request, each put away in one press");
  return {
    ...base,
    state: { ...base.state, repository: repository().root },
    draft: { sessions: () => sessionsStore([SESSION]) },
  };
}

export const s240DismissANeedsYouRow: Scenario = build();
