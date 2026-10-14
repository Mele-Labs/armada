// The pages under System, as a strip of tabs under the title bar. There only while System is the place, and
// nothing otherwise, so the window mounts it unconditionally. Not mounted by default (#2301).

import { Tabs } from "@armada/components";
import { SYSTEM_PAGES, deckTabOf } from "@armada/shell";

import "./deck-chrome.css";

export function SystemStrip({ showing, onGoTo }: {
  /** The surface that is up. */
  showing: string;
  /** Opens a surface, as the rail's rows do. */
  onGoTo: (surface: string) => void;
}) {
  if (deckTabOf(showing) !== "system") return null;
  return (
    <div className="deck-chrome">
      <nav className="deck-system" aria-label="System">
        <Tabs items={SYSTEM_PAGES.map((page) => ({ id: page.surface, label: page.label }))} value={showing} onChange={onGoTo} />
      </nav>
    </div>
  );
}
