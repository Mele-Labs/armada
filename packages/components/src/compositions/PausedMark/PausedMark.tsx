import { CirclePause } from "lucide-react";

import { Tooltip } from "../../primitives/Tooltip/Tooltip";

/**
 * A Job's pause, as a mark beside its status badge and never in place of it:
 * a Job at a review gate reads Needs review with this beside it. **An icon and
 * its tooltip, no phrase.** The caller says what the tooltip reads, because the
 * branch, who paused it and when are the Job's and not this mark's.
 */
export function PausedMark({ said }: { said: string }) {
  return (
    <Tooltip label={said}>
      <span className="armada-paused-mark" role="img" aria-label={said}>
        <CirclePause size={12} strokeWidth={2} aria-hidden />
      </span>
    </Tooltip>
  );
}
