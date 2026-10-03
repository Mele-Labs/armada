import { Fragment } from "react";
import { CircleDot, Eye, Scale, ShieldEllipsis, type LucideIcon } from "lucide-react";

import { Tooltip } from "../../primitives/Tooltip/Tooltip";

/**
 * A running step's parts in order — its Drones, its gate's Checks, its Judge,
 * and a person where one holds it — each **done** (filled), **now** (lit, and
 * moving while a machine is at it) or **next** (hollow), named on hover.
 *
 * The owner's annotation of 3 Oct 2026, `ouqa` (*"I had no idea it was running
 * checks"*). A mark on the state pill was *"too easy to miss still"*, and he
 * drew this track instead. **It is the card's one loop**, so a card carrying
 * one does not sweep (`design-system.md`, Motion); a person's part holds still.
 *
 * Glyphs from `packages/icons/icons.toml`: `circle-dot`, `shield-ellipsis`,
 * `scale` (both approved 3 Oct 2026), and `eye` for a person's turn.
 */
export type StepPhase = "drone" | "checks" | "judge" | "waiting";

export type StepPhasePart = {
  phase: StepPhase;
  /** Its name, beside its glyph — `Drones`, `Checks`, `Judge`, `You`. */
  name: string;
  /** What it says on hover and to a screen reader — `Checks running`, `Judge next`. */
  label: string;
  state: "done" | "now" | "next";
};

const GLYPH: Record<StepPhase, LucideIcon> = {
  drone: CircleDot,
  checks: ShieldEllipsis,
  judge: Scale,
  waiting: Eye,
};

export type StepPhaseTrackProps = { parts: readonly StepPhasePart[] };

export function StepPhaseTrack({ parts }: StepPhaseTrackProps) {
  return (
    <span className="armada-step-track" role="list" aria-label="Phases">
      {parts.map((part, index) => {
        const Icon = GLYPH[part.phase];
        return (
          <Fragment key={part.phase}>
            {index === 0 ? null : (
              <span className="armada-step-track__rule" data-state={part.state} aria-hidden="true" />
            )}
            <span role="listitem" className="armada-step-track__item">
              <Tooltip asChild label={part.label}>
                <span
                  className="armada-step-track__part"
                  data-phase={part.phase}
                  data-state={part.state}
                  role="img"
                  aria-label={part.label}
                >
                  <Icon size={12} strokeWidth={2} aria-hidden="true" />
                  <span aria-hidden="true">{part.name}</span>
                </span>
              </Tooltip>
            </span>
          </Fragment>
        );
      })}
    </span>
  );
}
