import { Clock, LogIn, ToggleLeft, Trash2, Unplug, type LucideIcon } from "lucide-react";
import type { LoadoutState, McpServerState } from "@armada/protocol";

import { Tooltip } from "../../primitives/Tooltip/Tooltip";

/**
 * Only the states that are not the usual one draw a mark: a server that failed, is still connecting, needs the person
 * to sign in, or was turned off, and a skill the person removed from this Mac. The agent's own report says `pending`
 * where a hosted Job's CLI says `connecting`.
 */
type Marked = LoadoutState | McpServerState | "removed";

const STATE_MARK: Partial<Record<Marked, { Glyph: LucideIcon; said: string }>> = {
  failed: { Glyph: Unplug, said: "Failed" },
  pending: { Glyph: Clock, said: "Connecting" },
  connecting: { Glyph: Clock, said: "Connecting" },
  needs_sign_in: { Glyph: LogIn, said: "Needs sign-in" },
  off: { Glyph: ToggleLeft, said: "Off" },
  removed: { Glyph: Trash2, said: "Removed" },
};

export function StateMark({ state }: { state: Marked | undefined }) {
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
