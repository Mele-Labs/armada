// What Fleet will do once a reply's close is read sentence by sentence, shown on fixtures. One Job whose last reply closes with three asks and
// two follow-ups: each ask is its own Needs you item, the approval the agent also stated is shown once, and the follow-ups it kept for
// itself are listed in Progress under Later. Fleet does not read a reply this way yet. The walk `needs-you-extracted` plays it.

import type { Session } from "@armada/screens/src/draft/sessions";
import { repository } from "@armada/screens/src/fixtures/repository";

import { holding } from "../holding";
import type { Scenario } from "../moment";
import { sessionsStore } from "../sessions/script";

const ago = (minutes: number) => new Date(Date.now() - minutes * 60_000).toISOString();

/** When the turn ended: the ids Fleet gives the asks it reads from the reply are `replied:<this>:<n>`. */
const ENDED = ago(3);

const QUESTION = "Should the registry be read once at start, or on every Studio open?";
const COMMAND = "pnpm vitest run packages/studios";
const RUN = "Run the Studio tests with this command.";
const APPROVE = "Approve #2714 and it lands.";
const LATER = ["I will file the slow fixture load once this lands.", "The Studio icon tooltip is still owed."];

const REPLY = [
  "The registry is read in Studio start and the Studio tests pass.",
  QUESTION,
  RUN,
  "```bash",
  COMMAND,
  "```",
  APPROVE,
  ...LATER,
].join("\n");

const SESSION: Session = {
  id: "ne-1",
  title: "Read the Studio registry on start",
  turn: { state: "idle" },
  lastTurnAt: ENDED,
  standing: "replied",
  rows: [
    { id: "ne-u", at: "14:02:00", kind: "message", from: { kind: "you" }, text: "Read the Studio registry when Studio starts." },
    { id: "ne-a", at: "14:19:40", kind: "message", from: { kind: "agent" }, text: REPLY },
  ],
  attachments: [],
  work: { proposal: { workflow: null } },
  // The agent called waiting_for with the approval alone. The reply's own copy of it is folded into that item, so it is not listed twice.
  waitingFor: [
    { id: `replied:${ENDED}:0`, text: QUESTION, since: ENDED, source: "replied", act: { kind: "answer", target: "" } },
    { id: `replied:${ENDED}:1`, text: RUN, since: ENDED, source: "replied", act: { kind: "run", target: COMMAND } },
    { id: "w-approve", text: APPROVE, since: ago(4), source: "agent", act: { kind: "approve_pr", target: "#2714" } },
  ],
  later: LATER.map((text) => ({ text, at: ENDED })),
};

function build(): Scenario {
  const base = holding("needs-you-extracted", "A reply that closes with three asks and two follow-ups: three Needs you items, the approval shown once, and the follow-ups under Later in Progress");
  return {
    ...base,
    state: { ...base.state, repository: repository().root },
    draft: { sessions: () => sessionsStore([SESSION]) },
  };
}

export const needsYouExtracted: Scenario = build();
