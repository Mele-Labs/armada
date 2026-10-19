// A Session whose thread holds, close together, an agent's message, the person's own typed message,
// an Approve from a walk and an answer to a waiting item. Over the `sessions` scenario. The walk
// `your-words-and-acts` plays it.

import type { Session } from "@armada/screens/src/draft/sessions";

import type { Scenario } from "../moment";
import { s200Sessions } from "./sessions";

const minutesAgo = (minutes: number) => new Date(Date.now() - minutes * 60_000).toISOString();

const told: Session = {
  id: "s26",
  title: "Store clock reply",
  turn: { state: "idle" },
  lastTurnAt: minutesAgo(2),
  rows: [
    { id: "y1", at: "09:55:40", kind: "message", from: { kind: "agent" }, text: "The clock is pinned in the store tests. I have put the page up to look at, and one question." },
    { id: "y2", at: "09:56:12", kind: "message", from: { kind: "you" }, text: "Keep the pin in the test module, not in the store itself." },
    { id: "y3", at: "09:57:23", kind: "message", from: { kind: "you" }, text: "Approved: http://localhost:5188/?walk=session-reply" },
    { id: "y4", at: "09:57:51", kind: "message", from: { kind: "you" }, text: "Re: Conflict detection: what happens on a real clash, and does Bridge show it\nYes bridge shows it. Also we need to account for when agents sessions are done that the index gets cleaned up. I would assume main getting updated also impacts this\nThis settles the item, so Fleet took it off your waiting list. Do not add it back." },
  ],
  attachments: [{ kind: "slot", slot: 3 }, { kind: "branch", name: "fix/pin-store-clock", slot: 3 }],
};

export const s234SessionYourWordsAndActs: Scenario = {
  ...s200Sessions,
  name: "session-your-words-and-acts",
  says: "A Session's thread with an agent message, the person's own, an Approve and an answer close together",
  draft: { sessions: (board) => s200Sessions.draft!.sessions!(board, [told]) },
};
