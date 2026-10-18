import { ArrowRight, ChevronRight, Link } from "lucide-react";

import { Tooltip } from "../../primitives/Tooltip/Tooltip";
import type { SessionThreadRow } from "./SessionThread";

const EYEBROW = { waits: "Waits on you", changed: "Condition changed", started: "Started" } as const;
// In the waiting Session's own thread the row names the Session it waits on.
const EYEBROW_WAITING = { waits: "Waits on", changed: "Condition changed", started: "Started" } as const;

/**
 * A Session that waits on this one, told in this one's thread: it was dispatched, its condition changed, or it started.
 * A press opens the waiting Session. The mark is the one the Work list draws on a waiting row.
 */
export function WaiterRow({ row, onOpen }: { row: Extract<SessionThreadRow, { kind: "waiter" }>; onOpen: (sessionId: string) => void }) {
  const eyebrow = row.waiting === true ? EYEBROW_WAITING : EYEBROW;
  const mark = row.waiting === true ? "Waits on that Job" : "Waits on this Job";
  const said = row.event === "changed" ? `${row.session.title}, condition changed` : `${row.session.title}, ${eyebrow[row.event].toLowerCase()}`;
  return (
    <li className="armada-session-waiter">
      <button type="button" className="armada-session-waiter__press" aria-label={said} onClick={() => onOpen(row.session.id)}>
        <span className="armada-session-waiter__line">
          <Tooltip label={mark}>
            <span role="img" aria-label={mark} className="armada-session-waiter__mark">
              <Link size={12} strokeWidth={2} aria-hidden />
            </span>
          </Tooltip>
          <span className="armada-session-waiter__eyebrow">{row.event === "changed" && row.by === "you" ? "You changed it" : eyebrow[row.event]}</span>
          <span className="armada-session-waiter__title">{row.session.title}</span>
          <span className="armada-session-waiter__at">{row.at}</span>
          <ChevronRight size={14} strokeWidth={2} aria-hidden className="armada-session-waiter__go" />
        </span>
        {row.says === undefined ? null : <span className="armada-session-waiter__facts">{row.says}</span>}
        {row.from === undefined || row.to === undefined ? null : (
          <span className="armada-session-waiter__facts">
            {row.from}
            <ArrowRight size={12} strokeWidth={2} aria-hidden />
            {row.to}
          </span>
        )}
      </button>
    </li>
  );
}
