import { Rewind } from "lucide-react";

import { Button } from "../../primitives/Button/Button";
import { Tooltip } from "../../primitives/Tooltip/Tooltip";

export type RetroPressProps = {
  /** The retro is being written. The mark moves and the press holds. */
  writing?: boolean;
  onPress: () => void;
};

/**
 * Writes a Session's retro and opens it on the Retros page. **An icon and its
 * tooltip, no label**, in a Session's header and on each row of the Sessions
 * list. It covers what happened since that Session's last retro.
 */
export function RetroPress({ writing = false, onPress }: RetroPressProps) {
  const said = writing ? "Writing the retro" : "Retro";
  return (
    <Tooltip label={said}>
      <Button variant="ghost" size="sm" className="armada-retro-press" aria-label={said} disabled={writing} data-writing={writing || undefined} onClick={onPress}>
        <Rewind size={16} strokeWidth={2} aria-hidden />
      </Button>
    </Tooltip>
  );
}
