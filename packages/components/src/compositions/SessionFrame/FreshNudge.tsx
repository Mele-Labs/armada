import { MoveRight } from "lucide-react";

import { Button } from "../../primitives/Button/Button";
import { Tooltip } from "../../primitives/Tooltip/Tooltip";
import { Inspect, InspectCard } from "../../primitives/Inspect/Inspect";
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
  return (
    <span className="armada-session-frame__fresh-host">
      <Inspect
        name="Fresh start"
        card={
          <InspectCard
            kind={{ Glyph: MoveRight, label: "Fresh start" }}
            where={`Started ${ageSaid(fresh.hours)} ago`}
            title="Start a new Job"
            facts={[{ label: "Context", value: `${tokensSaid(fresh.tokens)} tokens`, mono: true }]}
            keys={[{ key: "Enter", verb: "Keep open" }]}
          >
            <span className="armada-session-frame__fresh-acts">
              <Tooltip label="Start a new Job with a short handoff from this one">
                <Button variant="secondary" size="sm" disabled={fresh.busy === true} onClick={fresh.onFresh}>
                  New Job
                </Button>
              </Tooltip>
              {fresh.onFork === undefined ? null : (
                <Tooltip label="Start a new Job with this one's conversation">
                  <Button variant="secondary" size="sm" onClick={fresh.onFork}>
                    Fork
                  </Button>
                </Tooltip>
              )}
            </span>
          </InspectCard>
        }
      >
        <button type="button" className="armada-session-frame__fresh" aria-label="Fresh start" aria-busy={fresh.busy || undefined} data-busy={fresh.busy || undefined}>
          <MoveRight size={16} strokeWidth={2} aria-hidden />
        </button>
      </Inspect>
    </span>
  );
}
