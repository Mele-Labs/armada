// The window's tabs: where a person moves about. Drawn in the title bar's centre (`TitleBar`'s `places`), each tab one
// surface. System opens the page last used under it. #2301.

import { useLayoutEffect, useRef, useState } from "react";
import { SlidersHorizontal, type LucideIcon } from "lucide-react";
import { Tabs, usePlacesRoom } from "@armada/components";
import { DECK_TABS, SURFACES, deckTabOf } from "@armada/shell";

import { jobNameNotch, placesTake } from "./flight-take";
import { setTitleNotch } from "./title-notch";

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
  const drawn = tabs.map((tab) => tab.id).join(" ");
  // The strip stays on one line: where the room is narrower than the labels, each tab is drawn as its glyph. The
  // labels' width is read while they are drawn, so it is known when the row widens again.
  const room = usePlacesRoom();
  const nav = useRef<HTMLElement>(null);
  const [labels, setLabels] = useState<{ drawn: string; width: number }>();
  // `&places=` under `tight-type`: the glyph with small words, or the glyph alone with its name on hover (`flight-take.ts`).
  const places = placesTake();
  const iconsOnly = places === "icons" || (labels?.drawn === drawn && room !== undefined && labels.width > room);
  useLayoutEffect(() => {
    if (iconsOnly || nav.current === null) return;
    const width = Math.ceil(nav.current.getBoundingClientRect().width);
    if (labels?.drawn !== drawn || labels.width !== width) setLabels({ drawn, width });
  });
  const strip = (
    <nav ref={nav} className="deck-places" aria-label="Places" data-places={places}>
      <Tabs
        iconsOnly={iconsOnly}
        iconsWithLabels={places === "small"}
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
  // Under `&jobname=notch` the notch has a place after the Places for the open Job's state and name; it draws nothing while no Job puts its name there.
  if (!jobNameNotch()) return strip;
  return (
    <>
      {strip}
      <span ref={setTitleNotch} className="deck-places__job" />
    </>
  );
}
