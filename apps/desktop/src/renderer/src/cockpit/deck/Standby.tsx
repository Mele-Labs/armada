// What a panel shows while the deck powers on (`cockpit/boot.ts`), over its hidden body: the faint outlines of its own instruments, as
// a cockpit's bezels show before its displays light, or its own glyph dim in the middle (`power.css`, `data-standby-take` picks). It
// stands where the data will be, so the data replaces it in place. No text, nothing to press, and nothing drawn once the deck is live.

import type { CSSProperties } from "react";
import { Activity, ChartNoAxesGantt, ClipboardList, Gauge, Merge, NotebookText, Plane, Rows3, Send, type LucideIcon } from "lucide-react";

import { powering, usePowerOn } from "../boot";

export type StandbyKind = "jobs" | "bays" | "controls" | "queue" | "dispatch" | "autopilot" | "activity" | "timeline" | "ledger";

/** How many instrument outlines each panel shows, and its glyph from the registry. */
const PANELS: Record<StandbyKind, { shapes: number; glyph: LucideIcon }> = {
  jobs: { shapes: 7, glyph: ClipboardList },
  bays: { shapes: 6, glyph: Rows3 },
  controls: { shapes: 4, glyph: Gauge },
  queue: { shapes: 4, glyph: Merge },
  dispatch: { shapes: 1, glyph: Send },
  autopilot: { shapes: 2, glyph: Plane },
  activity: { shapes: 12, glyph: Activity },
  timeline: { shapes: 6, glyph: ChartNoAxesGantt },
  ledger: { shapes: 6, glyph: NotebookText },
};

export function Standby({ kind }: { kind: StandbyKind }) {
  const power = usePowerOn();
  if (!powering(power)) return null;
  const { shapes, glyph: Glyph } = PANELS[kind];
  return (
    <div className="deck-standby" data-standby={kind} aria-hidden="true">
      <div className="deck-standby__shapes">
        {Array.from({ length: shapes }, (_, at) => (
          <i key={at} style={{ "--i": at } as CSSProperties} />
        ))}
      </div>
      <Glyph className="deck-standby__glyph" size={28} strokeWidth={1.5} aria-hidden="true" />
    </div>
  );
}
