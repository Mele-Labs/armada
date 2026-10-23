// A sentence Fleet read out of a reply as an ask that is only a plan's summary line, beside a question the agent asked itself. The row read from
// the reply draws its own glyph and is marked Not a question in one of three takes. Fleet does not read a reply this way yet. The walks
// `not-a-question`, `not-a-question-thumbs` and `not-a-question-menu` play it; the first takes `?take=thumbs` or `?take=menu` too.

import type { Session } from "@armada/screens/src/draft/sessions";
import { repository } from "@armada/screens/src/fixtures/repository";

import { holding } from "./holding";
import type { Scenario } from "./moment";
import { sessionsStore } from "./sessions/script";

const ago = (minutes: number) => new Date(Date.now() - minutes * 60_000).toISOString();

/** When the turn ended: the ids Fleet gives the asks it reads from the reply are `replied:<this>:<n>`. */
const ENDED = ago(3);

const SUMMARY = "What you get: The Progress header keeps its name and its few icons: Ask for an update, Sketch and the approved mark.";
const QUESTION = "Should the header keep the approved mark, or drop it?";

const REPLY = ["The plan is recorded.", SUMMARY].join("\n");

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
    { id: "w-header", text: QUESTION, since: ago(4), source: "agent", act: { kind: "answer", target: "" }, options: [{ label: "Keep it" }, { label: "Drop it" }] },
  ],
};

export type Take = "button" | "thumbs" | "menu";

export function notAQuestionOver(name: string, take: Take): Scenario {
  const base = holding(name, "A plan's summary line read from a reply as an ask, beside a question the agent asked itself");
  return {
    ...base,
    state: { ...base.state, repository: repository().root },
    draft: { sessions: () => Object.assign(sessionsStore([SESSION]), { misreadAs: take }) },
  };
}
