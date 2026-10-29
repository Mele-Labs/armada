import { useId, useState } from "react";
import { ChevronDown, ChevronRight, Flag, RotateCcw, X } from "lucide-react";

import { Button } from "../../primitives/Button/Button";
import { Stamp } from "../../primitives/Stamp/Stamp";
import { Tooltip } from "../../primitives/Tooltip/Tooltip";
import type { SessionThreadRow } from "./SessionThread";

type NotStarted = Extract<SessionThreadRow, { kind: "not_started" }>;

/**
 * Fleet's line for a Job whose agent never came up, even on its second try: the cause, the agent's own last error lines folded behind a
 * chevron, and the two presses that end it, each in a column of its own. `onStartAgain` and `onReport` are absent where the host serves none.
 */
export function StartFailedRow({ row, onStartAgain, onReport }: { row: NotStarted; onStartAgain?: () => void; onReport?: (from: Element) => void }) {
  const [open, setOpen] = useState(false);
  const log = useId();
  return (
    <li className="armada-session-start-failed" role="region" aria-label="Failed to start">
      <Tooltip label="Failed to start">
        <span className="armada-session-start-failed__mark" role="img" aria-label="Failed to start">
          <X size={12} strokeWidth={2} aria-hidden />
        </span>
      </Tooltip>
      <span className="armada-session-start-failed__why">{row.why}</span>
      <span className="armada-session-start-failed__col">
        {row.log.length === 0 ? null : (
          <Tooltip label="Error lines">
            <Button variant="ghost" size="sm" iconOnly aria-label="Error lines" aria-expanded={open} aria-controls={log} onClick={() => setOpen(!open)}>
              {open ? <ChevronDown size={12} strokeWidth={2} aria-hidden /> : <ChevronRight size={12} strokeWidth={2} aria-hidden />}
            </Button>
          </Tooltip>
        )}
      </span>
      <span className="armada-session-start-failed__col">
        {onStartAgain === undefined ? null : (
          <Tooltip label="Start again">
            <Button variant="ghost" size="sm" iconOnly aria-label="Start again" onClick={onStartAgain}>
              <RotateCcw size={12} strokeWidth={2} aria-hidden />
            </Button>
          </Tooltip>
        )}
      </span>
      <span className="armada-session-start-failed__col">
        {onReport === undefined ? null : (
          <Tooltip label="Report">
            <Button variant="ghost" size="sm" iconOnly aria-label="Report" onClick={(event) => onReport(event.currentTarget)}>
              <Flag size={12} strokeWidth={2} aria-hidden />
            </Button>
          </Tooltip>
        )}
      </span>
      <Stamp still at={row.at} className="armada-session-start-failed__at" />
      {open ? <pre id={log} className="armada-session-start-failed__log">{row.log.join("\n")}</pre> : null}
    </li>
  );
}
