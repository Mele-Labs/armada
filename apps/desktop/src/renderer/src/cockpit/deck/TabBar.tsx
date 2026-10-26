// The window's tabs: where a person moves about. Drawn in the title bar's centre (`TitleBar`'s `places`), each tab one
// surface. System opens the page last used under it. #2301.

import { SlidersHorizontal, type LucideIcon } from "lucide-react";
import { Tabs } from "@armada/components";
import { DECK_TABS, SURFACES, deckTabOf } from "@armada/shell";

import "./deck-chrome.css";

/** What the Cockpit tab's pip says: a call is waiting while the window is somewhere else. */
const WAITING = "A call is waiting";

/** Each tab's glyph is its surface's in the roster; System opens several pages, so it has its own. */
function iconOf(tab: (typeof DECK_TABS)[number]): LucideIcon | undefined {
  return tab.id === "system" ? SlidersHorizontal : SURFACES.find((one) => one.id === tab.surface)?.icon;
}

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
  const tabs = DECK_TABS.filter((tab) => tab.id === "system" || !hidden.includes(tab.surface));
  // Each tab is its glyph, with its name in a tooltip, so the strip is one line at any width.
  return (
    <nav className="deck-places" aria-label="Places">
      <Tabs
        iconsOnly
        items={tabs.map((tab) => {
          const icon = iconOf(tab);
          return {
            id: tab.id,
            label: tab.label,
            ...(icon === undefined ? {} : { icon }),
            ...(tab.id === "cockpit" && waiting && place !== "cockpit" ? { alert: WAITING } : {}),
          };
        })}
        {...(place === undefined ? {} : { value: place })}
        onChange={(id) => {
          const tab = DECK_TABS.find((one) => one.id === id);
          // Pressing the place a person is in goes up to its list (an open Session closes), except the Cockpit, which is already the deck.
          if (tab !== undefined && (tab.id !== "cockpit" || tab.id !== place || away)) onGoTo(tab.surface);
        }}
      />
    </nav>
  );
}
