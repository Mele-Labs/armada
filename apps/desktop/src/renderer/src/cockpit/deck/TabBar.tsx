// The window's tabs: where a person moves about. Drawn in the title bar's centre (`TitleBar`'s `places`), each tab one
// surface. System opens the page last used under it. #2301.
//
// A right press on a tab, or the ContextMenu key or Shift+F10 on a focused one, opens a menu: Hide, then a View group with every place
// and whether it is drawn. A right press on the strip and on no tab opens the View group alone. Both write the owner's own layout for the
// `rail` region, the choice System → Layout shows, so a place hidden here is switched off there and back.

import { useRef, useState } from "react";
import { SlidersHorizontal, type LucideIcon } from "lucide-react";
import { DropdownMenu, Tabs, type DropdownMenuEntry } from "@armada/components";
import { DECK_TABS, SURFACES, deckTabOf, drawnTabs, useLayout, useLayouts } from "@armada/shell";

import "./deck-chrome.css";

/** What the Cockpit tab's pip says: a call is waiting while the window is somewhere else. */
const WAITING = "A call is waiting";

/** Why Cockpit and System have no Hide: System is how a hidden place comes back, and the Cockpit is the layout's own fixed place. */
const ALWAYS = "Always shown";

type DeckTab = (typeof DECK_TABS)[number];

/** Where the menu opens, and the tab it is for: none where the press was on the strip and on no tab. */
type MenuAt = { x: number; y: number; tab: DeckTab | undefined; from: HTMLElement | null };

/** Each tab's glyph is its surface's in the roster; System opens several pages, so it has its own. */
function iconOf(tab: DeckTab): LucideIcon | undefined {
  return tab.id === "system" ? SlidersHorizontal : SURFACES.find((one) => one.id === tab.surface)?.icon;
}

export function TabBar({ showing, onGoTo, waiting = false, away = false, hidden = [] }: {
  /** The surface that is up. */
  showing: string;
  /** A page opened from the Cockpit stands over it: pressing Cockpit then is the way back to it, not a press on the place it is already in. */
  away?: boolean;
  /** Surfaces nothing serves yet: not drawn, and not offered under View. What a layout hides is read here, and System is never hidden. */
  hidden?: readonly string[];
  /** Opens a surface. */
  onGoTo: (surface: string) => void;
  /** Whether a call waits on the person. The pip shows on Cockpit when the window is not already there. */
  waiting?: boolean;
}) {
  const layouts = useLayouts();
  const rail = useLayout("rail").all;
  const laidOff = rail.filter((one) => !one.visible).map((one) => one.id);
  const place = deckTabOf(showing);
  const tabs = drawnTabs([...hidden, ...laidOff]);
  const offered = drawnTabs(hidden);
  const nav = useRef<HTMLElement>(null);
  const [menu, setMenu] = useState<MenuAt | undefined>();
  const kept = useRef(false);

  const hideable = (tab: DeckTab) => tab.id !== "system" && rail.find((one) => one.id === tab.surface)?.hideable === true;
  const show = (tab: DeckTab, on: boolean) =>
    layouts.source.setOwn("rail", { ...layouts.own.regions.rail, hidden: on ? laidOff.filter((one) => one !== tab.surface) : [...laidOff, tab.surface] });
  /** The tab a press or a key landed on, by its place in the strip, which draws `tabs` in order. */
  const tabAt = (target: EventTarget) => {
    const button = (target as Element).closest?.<HTMLElement>('[role="tab"]') ?? null;
    const all = [...(nav.current?.querySelectorAll<HTMLElement>('[role="tab"]') ?? [])];
    return { button, tab: button === null ? undefined : tabs[all.indexOf(button)] };
  };
  const fixed = (tab: DeckTab) => (hideable(tab) ? {} : { disabled: true, hint: ALWAYS });

  const entries = (at: MenuAt): DropdownMenuEntry[] => [
    ...(at.tab === undefined ? [] : [{ kind: "item" as const, id: `hide:${at.tab.id}`, label: `Hide ${at.tab.label}`, ...fixed(at.tab) }, { kind: "separator" as const, id: "view-rule" }]),
    { kind: "label", id: "view", label: "View" },
    ...offered.map((tab): DropdownMenuEntry => ({ kind: "item", id: `view:${tab.id}`, label: tab.label, selected: tabs.includes(tab), ...fixed(tab) })),
  ];

  // Each tab is its glyph and then its name, small, so the strip stays one line.
  return (
    <nav
      ref={nav}
      className="deck-places"
      aria-label="Places"
      onContextMenu={(event) => {
        // The menu is portalled out of the strip, and React still bubbles its presses here.
        if (nav.current?.contains(event.target as Node) !== true) return;
        event.preventDefault();
        const { button, tab } = tabAt(event.target);
        setMenu({ x: event.clientX, y: event.clientY, tab, from: button });
      }}
      onKeyDown={(event) => {
        if (event.key !== "ContextMenu" && !(event.key === "F10" && event.shiftKey)) return;
        const { button, tab } = tabAt(event.target);
        if (button === null || nav.current?.contains(button) !== true) return;
        event.preventDefault();
        const box = button.getBoundingClientRect();
        setMenu({ x: box.left + box.width / 2, y: box.bottom, tab, from: button });
      }}
    >
      <Tabs
        icons
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
      {menu === undefined ? null : (
        <DropdownMenu
          triggerLabel="Places"
          at={{ x: menu.x, y: menu.y }}
          open
          entries={entries(menu)}
          onOpenChange={(open) => {
            if (open) return;
            setMenu(undefined);
            // Focus goes back once the pick is known: a View pick keeps the menu, and focus with it. A tab just hidden is gone, so focus goes to the one the strip holds.
            window.requestAnimationFrame(() => {
              if (kept.current) return void (kept.current = false);
              (menu.from?.isConnected === true ? menu.from : nav.current?.querySelector<HTMLElement>('[role="tab"][tabindex="0"]'))?.focus();
            });
          }}
          onSelect={(chosen) => {
            const [act, id] = chosen.split(":");
            const tab = offered.find((one) => one.id === id);
            if (tab === undefined || !hideable(tab)) return;
            if (act === "hide") return show(tab, false);
            // A View pick leaves the menu open, as the Jobs list's View does.
            show(tab, !tabs.includes(tab));
            kept.current = true;
            setMenu({ ...menu });
          }}
        />
      )}
    </nav>
  );
}
