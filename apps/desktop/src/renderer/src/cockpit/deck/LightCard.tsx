// The shell of a light on the merge queue that opens a card: the inspect card, on the light. Hovering or focusing the light opens it, a
// press pins it, and Esc or a press outside puts it away, as every card does. Main's light is the one in it: its card holds main's red and
// the checkout it carries. #2301.

import type { ReactNode } from "react";
import { GitMerge, type LucideIcon } from "lucide-react";
import { Inspect, InspectCard } from "@armada/components";

export function LightCard({
  said,
  chip,
  mark,
  state,
  summoned = false,
  to,
  onSummoned,
  children,
}: {
  /** The light's name, and the card's. */
  said: string;
  /** What the chip draws inside itself. */
  chip: ReactNode;
  /** Attributes the chip's styling reads: main's, or a tone. */
  mark: Record<string, string | true>;
  /** Main's state in the card's band: its glyph, its words and the status token it takes. */
  state: { Glyph: LucideIcon; said: string; tone: string };
  /** A call asked for the card to open: it is pinned and focus goes to the part of it `to` names, and `onSummoned` lets go of the ask. */
  summoned?: boolean;
  to?: string;
  onSummoned?: () => void;
  /** The card's body, given the way to put the card away. */
  children: (away: () => void) => ReactNode;
}) {
  return (
    <Inspect
      name={said}
      summoned={summoned}
      {...(to === undefined ? {} : { to })}
      {...(onSummoned === undefined ? {} : { onSummoned })}
      card={({ close }) => (
        <InspectCard className="deck-main-card" kind={{ Glyph: GitMerge, label: "Main" }} title="main" state={state} keys={[{ key: "Enter", verb: "Keep open" }]} width="wide">
          {children(close)}
        </InspectCard>
      )}
    >
      <button type="button" className="deck-chip" {...mark} aria-label={said}>
        {chip}
      </button>
    </Inspect>
  );
}
