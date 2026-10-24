// What a panel shows while the deck powers on (`cockpit/boot.ts`), over its hidden body: its own glyph from the registry, dim in the
// middle, lit once as the panel comes online (`power.css`). No text, nothing to press, and nothing drawn once the deck is live.

import { Activity, ChartNoAxesGantt, ClipboardList, Gauge, Merge, NotebookText, Plane, Rows3, Send, type LucideIcon } from "lucide-react";

import { powering, usePowerOn } from "../boot";

export type StandbyKind = "jobs" | "bays" | "controls" | "queue" | "dispatch" | "autopilot" | "activity" | "timeline" | "ledger";

/** Each panel's glyph, from the registry. */
const GLYPHS: Record<StandbyKind, LucideIcon> = {
  jobs: ClipboardList,
  bays: Rows3,
  controls: Gauge,
  queue: Merge,
  dispatch: Send,
  autopilot: Plane,
  activity: Activity,
  timeline: ChartNoAxesGantt,
  ledger: NotebookText,
};

export function Standby({ kind }: { kind: StandbyKind }) {
  const power = usePowerOn();
  if (!powering(power)) return null;
  const Glyph = GLYPHS[kind];
  return (
    <div className="deck-standby" data-standby={kind} aria-hidden="true">
      <Glyph className="deck-standby__glyph" size={28} strokeWidth={1.5} aria-hidden="true" />
    </div>
  );
}
