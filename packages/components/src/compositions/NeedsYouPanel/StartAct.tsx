import { LoaderCircle } from "lucide-react";

import { Button } from "../../primitives/Button/Button";
import { Tooltip } from "../../primitives/Tooltip/Tooltip";
import type { LedgerEntry } from "../SessionLedger/SessionLedger";

/**
 * Start at a held Job's row end, in a column of its own width so a row with one lines up with the next; while the request is read, Starting.
 * Drawn by the Needs you row and by the Started row in its parent's thread, so the two read and press the same. `name` is the button's own name
 * where a bare "Start" would not say which Job.
 */
export function StartAct({ start, name }: { start: NonNullable<LedgerEntry["start"]>; name?: string }) {
  return (
    <span className="armada-needs-panel__start">
      {start.starting ? (
        <span className="armada-needs-panel__starting">
          <Tooltip label="Starting">
            <span className="armada-session-mark" role="img" aria-label="Starting">
              <LoaderCircle size={12} strokeWidth={2} aria-hidden />
            </span>
          </Tooltip>
          <span aria-hidden>Starting…</span>
        </span>
      ) : (
        <Button variant="secondary" size="sm" data-aside="" {...(name === undefined ? {} : { "aria-label": name })} onClick={start.onStart}>
          Start
        </Button>
      )}
    </span>
  );
}
