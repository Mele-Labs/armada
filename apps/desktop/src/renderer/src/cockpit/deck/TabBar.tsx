// The window's tabs: where a person moves about. Drawn in the title bar's centre (`TitleBar`'s `places`), each tab one
// surface. System opens the strip of pages under the bar. #2301.

import { Tabs } from "@armada/components";
import { DECK_TABS, deckTabOf } from "@armada/shell";

import "./deck-chrome.css";

/** What the Cockpit tab's pip says: a call is waiting while the window is somewhere else. */
const WAITING = "A call is waiting";

export function TabBar({ showing, onGoTo, waiting = false, away = false, hidden = [] }: {
  /** The surface that is up. */
  showing: string;
  /** A page opened from the Cockpit stands over it: pressing Cockpit then is the way back to it, not a press on the place it is already in. */
  away?: boolean;
  /** Surfaces left off: the tab of a surface that is hidden is not drawn. System is never hidden. */
  hidden?: readonly string[];
  /** Opens a surface. */
  onGoTo: (surface: string) => void;
  /** Whether a call waits on the person. The pip shows on Cockpit when the window is not already there. */
  waiting?: boolean;
}) {
  const place = deckTabOf(showing);
  return (
    <nav className="deck-places" aria-label="Places">
      <Tabs
        items={DECK_TABS.filter((tab) => tab.id === "system" || !hidden.includes(tab.surface)).map((tab) => ({
          id: tab.id,
          label: tab.label,
          ...(tab.id === "cockpit" && waiting && place !== "cockpit" ? { alert: WAITING } : {}),
        }))}
        {...(place === undefined ? {} : { value: place })}
        onChange={(id) => {
          const tab = DECK_TABS.find((one) => one.id === id);
          if (tab !== undefined && (tab.id !== place || (tab.id === "cockpit" && away))) onGoTo(tab.surface);
        }}
      />
    </nav>
  );
}
