// The Cockpit window's tabs: where a person moves about once there is no rail. Drawn in the title bar's centre
// (`TitleBar`'s `places`), each tab one surface. System opens the strip of pages under the bar. Not mounted by
// default: the window that has no rail passes it (#2301).

import { Tabs } from "@armada/components";
import { DECK_TABS, deckTabOf } from "@armada/shell";

import "./deck-chrome.css";

/** What the Cockpit tab's pip says: a call is waiting while the window is somewhere else. */
const WAITING = "A call is waiting";

export function TabBar({ showing, onGoTo, waiting = false }: {
  /** The surface that is up. */
  showing: string;
  /** Opens a surface, as the rail's rows do. */
  onGoTo: (surface: string) => void;
  /** Whether a call waits on the person. The pip shows on Cockpit when the window is not already there. */
  waiting?: boolean;
}) {
  const place = deckTabOf(showing);
  return (
    <nav className="deck-places" aria-label="Places">
      <Tabs
        items={DECK_TABS.map((tab) => ({
          id: tab.id,
          label: tab.label,
          ...(tab.id === "cockpit" && waiting && place !== "cockpit" ? { alert: WAITING } : {}),
        }))}
        {...(place === undefined ? {} : { value: place })}
        onChange={(id) => {
          const tab = DECK_TABS.find((one) => one.id === id);
          if (tab !== undefined && tab.id !== place) onGoTo(tab.surface);
        }}
      />
    </nav>
  );
}
