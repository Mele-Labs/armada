import { MoveRight } from "lucide-react";

import { Button } from "../../primitives/Button/Button";
import { Tooltip } from "../../primitives/Tooltip/Tooltip";
import { StopCard, useCardOpen } from "../StopCard/StopCard";
import { tokensSaid } from "./ContextFigure";

/** A Session that is big or old, as figures already said, and the two acts offered on it. */
export type FreshStart = {
  /** The live tokens in its context window. */
  tokens: number;
  /** Whole hours since it started. */
  hours: number;
  /** The old Session is writing its handoff: the mark pulses and New Session waits. */
  busy?: boolean;
  /** Starts a new Session that opens on a short handoff from this one. */
  onFresh: () => void;
  /** Starts a new Session from this one's conversation. Absent where Fleet offers no fork of a live Session. */
  onFork?: () => void;
};

/** 26 hours as "26h", 53 hours as "2d". */
const ageSaid = (hours: number): string => (hours >= 48 ? `${Math.floor(hours / 24)}d` : `${hours}h`);

/**
 * A quiet mark beside the context figure once a Session is big or old. **Hover or focus opens its card, a
 * press keeps it open, and Esc or a press outside closes it**, as the context figure's does.
 */
export function FreshNudge({ fresh }: { fresh: FreshStart }) {
  const card = useCardOpen();
  return (
    <span className="armada-session-frame__fresh-host" {...card.bind}>
      <button type="button" className="armada-session-frame__fresh" aria-label="Fresh start" aria-expanded={card.open} aria-busy={fresh.busy || undefined} data-busy={fresh.busy || undefined} onClick={card.toggle}>
        <MoveRight size={16} strokeWidth={2} aria-hidden />
      </button>
      {card.open ? (
        <StopCard
          className="armada-session-frame__fresh-card"
          role="dialog"
          aria-label="Fresh start"
          pill={`${tokensSaid(fresh.tokens)} context`}
          where={`Started ${ageSaid(fresh.hours)} ago`}
          title="Fresh start"
        >
          <span className="armada-session-frame__fresh-acts">
            <Tooltip label="Start a new Session with a short handoff from this one">
              <Button variant="secondary" size="sm" disabled={fresh.busy === true} onClick={fresh.onFresh}>
                New Session
              </Button>
            </Tooltip>
            {fresh.onFork === undefined ? null : (
              <Tooltip label="Start a new Session with this one's conversation">
                <Button variant="secondary" size="sm" onClick={fresh.onFork}>
                  Fork
                </Button>
              </Tooltip>
            )}
          </span>
        </StopCard>
      ) : null}
    </span>
  );
}
