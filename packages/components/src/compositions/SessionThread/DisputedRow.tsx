import { ChevronRight, MessageSquareWarning } from "lucide-react";

import { Stamp } from "../../primitives/Stamp/Stamp";
import { Tooltip } from "../../primitives/Tooltip/Tooltip";
import type { SessionThreadRow } from "./SessionThread";

type Disputed = Extract<SessionThreadRow, { kind: "disputed" }>;

export const DISPUTED = "Disputed";

const SPOTTED: Record<Disputed["spotted"], string> = {
  called: "Said by the agent",
  read: "Read by Fleet from the agent's replies",
};

/** The mark Fleet's card draws once the agent disputed it. */
export function DisputedMark() {
  return (
    <Tooltip label={DISPUTED}>
      <span className="armada-session-disputed__mark" role="img" aria-label={DISPUTED}>
        <MessageSquareWarning size={12} strokeWidth={2} aria-hidden />
      </span>
    </Tooltip>
  );
}

/** The agent disputed a notice of Fleet's: one line in Fleet's voice, with the review Job as a press where one was started. */
export function DisputedRow({ row, onOpen }: { row: Disputed; onOpen: (sessionId: string) => void }) {
  const { review } = row;
  return (
    <li className="armada-session-disputed" role="region" aria-label="Notice disputed">
      <DisputedMark />
      <span className="armada-session-disputed__why">{row.why}</span>
      <span className="armada-session-disputed__spotted">{SPOTTED[row.spotted]}</span>
      {review === undefined ? null : (
        <button type="button" className="armada-session-disputed__review" aria-label="Open the review job" onClick={() => onOpen(review)}>
          Review job
          <ChevronRight size={12} strokeWidth={2} aria-hidden />
        </button>
      )}
      <Stamp still at={row.at} className="armada-session-disputed__at" />
    </li>
  );
}
