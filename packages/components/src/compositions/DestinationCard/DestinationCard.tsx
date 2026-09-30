// A card on a destination: a head strip naming what the card holds, then what
// it holds. Pulse drew it first, as a private card of its own; Overview draws
// the same one, so there is one card and not two that drift.

import { ChevronRight } from "lucide-react";
import type { ReactNode } from "react";

import { GuideMark } from "../GuideMark/GuideMark";
import type { Guide } from "../../guides/guide";

export type DestinationCardProps = {
  /** The card's name, in its head. Also the section's accessible name. */
  label: string;
  /** The `?` beside the name, where the name is Armada's own vocabulary. */
  guide?: Guide;
  /**
   * What the card says at its trailing edge: a figure, a read time, a filter,
   * an act. It wraps rather than clipping, for a narrow sheet.
   */
  trailing?: ReactNode;
  /**
   * The destination this card summarises. Set, the whole head is a press that
   * goes there, with a `chevron-right` at its trailing end.
   *
   * **The press lies behind the head, never around it.** A head holds a
   * `GuideMark` and can hold controls in `trailing`, and a button may not hold
   * a button, so the press is a sibling drawn under the name and the trailing
   * slot. What is in `trailing` keeps its own presses; the space between them
   * falls through to the head's.
   */
  onOpen?: () => void;
  children: ReactNode;
};

/** `chevron-right`'s *goes to* usage: 12px, trailing, at the text's own size. */
const CHEVRON = 12;

export function DestinationCard({ label, guide, trailing, onOpen, children }: DestinationCardProps) {
  return (
    // **`armada-glass` beside its own class**, which is how a card on the
    // canvas takes the surface rather than copying it — `glass.css`. Without
    // it the card read flat beside the left column's panels, which have had
    // the treatment since #1260. The owner, 29 Sep 2026.
    <section className="armada-destination-card armada-glass" aria-label={label}>
      <div className="armada-destination-card__head" data-opens={onOpen === undefined ? undefined : ""}>
        {onOpen === undefined ? null : (
          <button
            type="button"
            className="armada-destination-card__press"
            aria-label={`Open ${label}`}
            onClick={onOpen}
          />
        )}
        <h3 className="armada-destination-card__title">{label}</h3>
        {guide === undefined ? null : <GuideMark guide={guide} />}
        <span className="armada-destination-card__trail">{trailing}</span>
        {onOpen === undefined ? null : (
          <ChevronRight
            className="armada-destination-card__chevron"
            size={CHEVRON}
            strokeWidth={2}
            aria-hidden="true"
          />
        )}
      </div>
      <div className="armada-destination-card__body">{children}</div>
    </section>
  );
}
