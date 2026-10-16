// A card for each concern a judge raised at a step. Nothing reaches the agent until the person presses Send to session; a judge
// never blocks. Drawn only while there is one: the slot is empty otherwise.

import { Scale } from "lucide-react";
import { Button, Tooltip } from "@armada/components";
import type { WorkConcern } from "@armada/screens/src/draft/sessions";

import { SvPanel, type Place } from "./SvPanel";

export function ConcernsPanel({ concerns, place, onSend, onDismiss }: { concerns: readonly WorkConcern[]; place: Place; onSend: (id: string) => void; onDismiss: (id: string) => void }) {
  return (
    <SvPanel label="Concerns" area="concerns" place={place} state="asks">
      <ul className="sv-concerns">
        {concerns.map((one) => (
          <li key={one.id} className="sv-concern">
            <header className="sv-concern__head">
              <Tooltip label="Judge">
                <span className="sv-concern__judge" role="img" aria-label={`Judge on ${one.step}`}>
                  <Scale size={12} strokeWidth={2} aria-hidden />
                </span>
              </Tooltip>
              <code>{one.step}</code>
            </header>
            <p className="sv-concern__text">{one.text}</p>
            <div className="sv-concern__acts">
              <Button size="sm" variant="secondary" onClick={() => onSend(one.id)}>
                Send to session
              </Button>
              <Button size="sm" variant="ghost" onClick={() => onDismiss(one.id)}>
                Dismiss
              </Button>
            </div>
          </li>
        ))}
      </ul>
    </SvPanel>
  );
}
