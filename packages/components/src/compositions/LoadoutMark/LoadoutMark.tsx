import { Layers } from "lucide-react";
import type { LoadoutSummary } from "@armada/protocol";

import { Button } from "../../primitives/Button/Button";
import { Inspect, InspectCard, InspectSection, type InspectFact } from "../../primitives/Inspect/Inspect";
import { countSaid, useSaid } from "../../loadout";
import { timeSaid, useTimeFormat } from "../../time-format";

/** The uses the card lists from the latest turn; the rest are counted. */
const SHOWN = 6;

export type LoadoutMarkProps = {
  summary: LoadoutSummary;
  /** Opens the whole Loadout, on the latest turn. */
  onOpen: () => void;
};

function LoadoutCard({ summary, onOpen }: LoadoutMarkProps) {
  const turn = summary.latest_turn;
  const format = useTimeFormat();
  const used: InspectFact[] = (turn?.used ?? []).slice(0, SHOWN).map((one) => ({ label: [one.name, countSaid(one)].filter((part) => part !== undefined).join(" "), value: useSaid(one) }));
  const more = (turn?.used.length ?? 0) - SHOWN;
  return (
    <InspectCard
      kind={{ Glyph: Layers, label: "Loadout" }}
      {...(turn === undefined ? {} : { where: timeSaid(turn.started_at, format) })}
      facts={[
        { label: "Knows", value: summary.knows, mono: true },
        { label: "Told", value: summary.told, mono: true },
      ]}
      keys={[{ key: "Enter", verb: "Show all" }]}
    >
      <InspectSection heading="Latest turn" facts={more > 0 ? [...used, { label: `${more} more`, value: "" }] : used} />
      <span className="armada-loadout-mark__acts">
        <Button variant="secondary" size="sm" onClick={onOpen}>
          Show all
        </Button>
      </span>
    </InspectCard>
  );
}

/**
 * What the Session's agent carries, as one quiet mark in its head. **Hover or focus opens its card**, which counts
 * what it knows and was told and lists what the latest turn used. **A press opens the whole Loadout**, so the press is
 * an act and does not pin the card.
 */
export function LoadoutMark({ summary, onOpen }: LoadoutMarkProps) {
  return (
    <span className="armada-loadout-mark">
      <Inspect name="Loadout" pin={false} card={<LoadoutCard summary={summary} onOpen={onOpen} />}>
        <button type="button" className="armada-loadout-mark__button" aria-label="Loadout" onClick={onOpen}>
          <Layers size={16} strokeWidth={2} aria-hidden />
        </button>
      </Inspect>
    </span>
  );
}
