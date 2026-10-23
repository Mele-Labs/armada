// Two sentences Fleet read out of a reply as asks that are only a plan's summary lines, beside a question the agent asked itself. A row read from a reply
// draws its own glyph, and ends in thumbs that say whether Fleet read it right; its menu offers the same two. Fleet does not read a reply this way yet. The walk `not-a-question` plays it.

import type { Session } from "@armada/screens/src/draft/sessions";
import { repository } from "@armada/screens/src/fixtures/repository";

import { holding } from "../holding";
import type { Scenario } from "../moment";
import { sessionsStore } from "../sessions/script";

const ago = (minutes: number) => new Date(Date.now() - minutes * 60_000).toISOString();

/** When the turn ended: the ids Fleet gives the asks it reads from the reply are `replied:<this>:<n>`. */
const ENDED = ago(3);

const SUMMARY = "What you get: The Progress header keeps its name and its few icons: Ask for an update, Sketch and the approved mark.";
const NEXT = "Next: The header's tests are rewritten to read the three icons by name.";
const QUESTION = "Should the header keep the approved mark, or drop it?";

const REPLY = ["The plan is recorded.", SUMMARY, NEXT].join("\n");

const SESSION: Session = {
  id: "naq-1",
  title: "Trim the Progress header",
  turn: { state: "idle" },
  lastTurnAt: ENDED,
  standing: "replied",
  rows: [
    { id: "naq-u", at: "14:02:00", kind: "message", from: { kind: "you" }, text: "Trim the Progress header to its few icons." },
    { id: "naq-a", at: "14:19:40", kind: "message", from: { kind: "agent" }, text: REPLY },
  ],
  attachments: [],
  work: { proposal: { workflow: null } },
  waitingFor: [
    { id: `replied:${ENDED}:0`, text: SUMMARY, since: ENDED, source: "replied" },
    { id: `replied:${ENDED}:1`, text: NEXT, since: ENDED, source: "replied" },
    { id: "w-header", text: QUESTION, since: ago(4), source: "agent", act: { kind: "answer", target: "" }, options: [{ label: "Keep it" }, { label: "Drop it" }] },
  ],
};

function build(): Scenario {
  const base = holding("not-a-question", "Two plan summary lines read from a reply as asks, beside a question the agent asked itself");
  return {
    ...base,
    state: { ...base.state, repository: repository().root },
    draft: { sessions: () => sessionsStore([SESSION]) },
  };
}

export const notAQuestion: Scenario = build();
