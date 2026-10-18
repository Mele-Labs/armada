import { Clock, Unplug, type LucideIcon } from "lucide-react";
import type { LoadoutState } from "@armada/protocol";

import { Tooltip } from "../../primitives/Tooltip/Tooltip";

/** Only the states that are not the usual one draw a mark: a server that failed or is still connecting. */
const STATE_MARK: Partial<Record<LoadoutState, { Glyph: LucideIcon; said: string }>> = {
  failed: { Glyph: Unplug, said: "Failed" },
  pending: { Glyph: Clock, said: "Connecting" },
};

export function StateMark({ state }: { state: LoadoutState | undefined }) {
  const mark = state === undefined ? undefined : STATE_MARK[state];
  if (mark === undefined) return null;
  return (
    <Tooltip label={mark.said}>
      <span className="armada-loadout__state" role="img" aria-label={mark.said} data-state={state}>
        <mark.Glyph size={12} strokeWidth={2} aria-hidden />
      </span>
    </Tooltip>
  );
}
