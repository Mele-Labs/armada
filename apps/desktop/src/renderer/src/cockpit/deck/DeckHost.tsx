// What the flight deck mounts at the window, outside the Cockpit's body: the places in the title bar, and the keys, which are the
// window's wherever the person is (the page chord and the next call work over a Job's page too). **Mounted only while the preview
// switch is on**, so with it off none of this is in the window. The body is `Deck.tsx`; the state is `store.ts`, a module, so
// this, the body and the keys are one thing and a body that unmounts while a page is open comes back exact. #2301.

import { useEffect, useMemo, useState } from "react";
import { isPressed, pressedDigit, pressedSlot } from "@armada/components";
import { DECK_TABS, SURFACE, SYSTEM_PAGES, deckTabOf } from "@armada/shell";
import { holdsText } from "@armada/screens/src/keys";

import { useItems, type Hosts } from "../../Dashboard";
import type { BridgeState } from "../../../../shared/bridge";
import type { RepositorySummary } from "@armada/protocol";
import type { CallView } from "@armada/jobs/draft/calls";
import type { NowView } from "@armada/jobs/draft/now";
import { holdFor } from "../Cockpit";
import { TILES } from "./Deck";
import { DeckKeySheet, armChord, cycleFrom, focusPanel, panelAt, remember, stepBy, stepIn, useDeckKeys } from "./keys";
import { PAGE_LETTERS, panelOfSlot, type PageLetter } from "./keys-model";
import { rememberSystemPage, systemPageOf } from "./preview";
import { SystemStrip } from "./SystemStrip";
import { TabBar } from "./TabBar";
import { callTrail, deckNow, goBack, incomingOf, leaveAutopilot, listOf, moveCursor, openCall, release, select, selOfSubject, setHere, setView, useDeck } from "./store";
import { useShip } from "./ship";

const NO_HOSTS: Hosts = { onOpen: () => {}, onOpenSession: () => {}, onOpenLink: () => {} };

type Where = {
  state: BridgeState;
  now: number;
  picked: RepositorySummary | null;
  nowViews: Readonly<Record<string, CallView>> | undefined;
  nows: Readonly<Record<string, NowView>> | undefined;
};

const NONE: ReadonlySet<string> = new Set();

/**
 * The places, for the title bar's centre. A call waiting while the window is not on the Cockpit pips the Cockpit tab, as the
 * classic Cockpit's toast does not cover (the toast speaks on the other tabs and this stays on them). **Pressing Cockpit on a page
 * that was opened from the deck goes back**, so the deck returns exactly as it was left; pressing another place from over the
 * Cockpit replaces the layers, so Back from it returns to the Cockpit.
 */
export function DeckPlaces({ state, picked, nowViews, showing, away, onGoTo, onBack, onReplaceNext }: Pick<Where, "state" | "picked" | "nowViews"> & {
  showing: string;
  /** A page opened from the deck stands over the Cockpit: the Job's page, the composer, a report. */
  away: boolean;
  onGoTo: (surface: string) => void;
  onBack: () => void;
  onReplaceNext: () => void;
}) {
  const deck = useDeck();
  const calls = useItems("command-central", state, picked, nowViews, NO_HOSTS, NONE);
  const waiting = incomingOf(calls, deck.held, deck.visited).length > 0;
  useEffect(() => {
    if (deckTabOf(showing) === "system") rememberSystemPage(showing);
  }, [showing]);
  const go = (surface: string) => {
    if (surface === SURFACE.overview && away) return onBack();
    if (away) onReplaceNext();
    // The System tab opens the page last used there.
    onGoTo(surface === DECK_TABS.find((tab) => tab.id === "system")!.surface ? systemPageOf(SYSTEM_PAGES.map((page) => page.surface), surface) : surface);
  };
  return <TabBar showing={showing} away={away} onGoTo={go} waiting={waiting} />;
}

/** The strip of System pages under the bar, there only while System is the place. */
export function DeckSystem({ showing, onGoTo }: { showing: string; onGoTo: (surface: string) => void }) {
  return <SystemStrip showing={showing} onGoTo={onGoTo} />;
}

/**
 * The deck's keys and housekeeping, drawn as nothing but the key sheet. Every key is asked of the keymap (`isPressed`,
 * `pressedSlot`, `pressedDigit`) and none is spelled here.
 */
export function DeckHost({ state, now, picked, nowViews, nows, here, openJobId, onGoTo }: Where & {
  /** The Cockpit is what is on screen, with no page over it. */
  here: boolean;
  openJobId: string | null;
  onGoTo: (surface: string) => void;
}) {
  const ship = useShip({ ...NO_HOSTS, state, now, picked, nowViews, nows }, false);
  const deck = useDeck();
  useEffect(() => setHere(here), [here]);

  // An answered call is held out until Fleet stops carrying it, or `holdFor` has passed.
  useEffect(() => {
    if (deck.held.size === 0) return;
    const live = new Set(ship.calls.map((one) => one.key));
    const gone = [...deck.held].filter(([key, since]) => !live.has(key) || Date.now() - since >= holdFor.ms).map(([key]) => key);
    if (gone.length > 0) return release(gone);
    const next = Math.min(...deck.held.values()) + holdFor.ms - Date.now();
    const timer = window.setTimeout(() => release([]), Math.max(0, next));
    return () => window.clearTimeout(timer);
  }, [deck.held, ship.calls]);

  const incoming = incomingOf(ship.calls, deck.held, deck.visited);
  const listed = useMemo(() => listOf(ship, deck), [ship, deck.only, deck.needs, deck.checks, deck.held]);
  const [keySheet, setKeySheet] = useState(false);
  useEffect(() => {
    const noted = (event: FocusEvent) => remember(event.target);
    document.addEventListener("focusin", noted);
    return () => document.removeEventListener("focusin", noted);
  }, []);
  /** Focus goes where the key said once the stage has drawn what it was asked to. */
  const focusSoon = (id: Parameters<typeof focusPanel>[0]) => window.requestAnimationFrame(() => focusPanel(id, deckNow().cursor));
  /** Pick a row of the list: the cursor moves with focus, and the row is brought into view. */
  const rowFocus = (key: string) => {
    moveCursor(key);
    const row = () => document.querySelector<HTMLElement>(`[data-row="${CSS.escape(key)}"]`);
    if (row() !== null) row()!.focus();
    else window.requestAnimationFrame(() => row()?.focus());
  };

  const go = (place: (typeof PAGE_LETTERS)[PageLetter]) => {
    const tab = DECK_TABS.find((one) => one.id === place)!;
    onGoTo(place === "system" ? systemPageOf(SYSTEM_PAGES.map((page) => page.surface), tab.surface) : tab.surface);
  };
  // The next call, or forward along the ones already opened; Shift goes back along them. A call answered since is passed over.
  const walkCalls = (by: 1 | -1) => {
    const live = (key: string) => ship.calls.find((one) => one.key === key && !deckNow().held.has(key));
    const order = by === -1 ? callTrail.keys.map((_, at) => at).filter((at) => at < (callTrail.at < 0 ? callTrail.keys.length : callTrail.at)).reverse() : callTrail.keys.map((_, at) => at).filter((at) => callTrail.at >= 0 && at > callTrail.at);
    const at = order.find((one) => live(callTrail.keys[one]!) !== undefined);
    let to = at === undefined ? undefined : live(callTrail.keys[at]!);
    if (at !== undefined) callTrail.at = at;
    else if (by === 1 && incoming[0] !== undefined) {
      to = incoming[0];
      callTrail.keys.push(to.key);
      callTrail.at = callTrail.keys.length - 1;
    }
    if (to === undefined) return;
    openCall(ship, to);
    if (!deckNow().here) onGoTo(SURFACE.overview);
  };

  useDeckKeys((event) => {
    if (event.metaKey || event.ctrlKey || event.altKey) return;
    const state = deckNow();
    const target = event.target as HTMLElement;
    if (holdsText(event.target) && target.getAttribute("role") !== "switch") {
      // The first Esc leaves the box, the next one acts.
      if (isPressed("close", event) && state.here && state.sel?.kind !== "call" && target.closest(".deck") !== null) {
        event.preventDefault();
        target.blur();
      }
      return;
    }
    if (target.closest?.('[role="menu"]') != null) return;
    // The next call, and with Shift the one before: `call_recall`'s own key, read unshifted.
    const recall = isPressed("call_recall", event) || (event.shiftKey && isPressed("call_recall", { key: event.key.toLowerCase(), metaKey: false, ctrlKey: false, altKey: false }));
    if (recall) {
      event.preventDefault();
      return walkCalls(event.shiftKey ? -1 : 1);
    }
    // Two keys: the page in front reads neither (`armChord` takes the second before it can). Not over a Job's page, where `g` is its own.
    if (isPressed("deck_page", event) && openJobId === null) {
      event.preventDefault();
      return armChord(Object.fromEntries(Object.entries(PAGE_LETTERS).map(([letter, place]) => [letter, () => go(place)])));
    }
    if (!state.here) return;
    if (isPressed("key_sheet", event)) {
      event.preventDefault();
      return setKeySheet(true);
    }
    // A call in front takes the bare digits and letters it names, and the deck stands down for it.
    if (state.sel?.kind === "call") return;
    const panel = panelAt(target);
    if (event.key === "Tab" || event.key === "F6") {
      // Tab goes round the panels from any of them but the stage while it holds a page of its own, whose controls it is left to reach; F6 always.
      if (event.key === "Tab" && (panel === undefined || (panel.id === "stage" && state.sel !== undefined))) return;
      event.preventDefault();
      return focusPanel(cycleFrom(panel?.id, event.shiftKey ? -1 : 1), state.cursor);
    }
    const digit = pressedDigit("deck_tile", event);
    if (digit !== null) {
      const tile = TILES[digit - 1];
      if (tile === undefined) return;
      event.preventDefault();
      if (tile.built) tile.press();
      return;
    }
    const slot = pressedSlot("deck_panel", event);
    const goes = slot === -1 ? undefined : panelOfSlot(slot);
    if (goes !== undefined) {
      event.preventDefault();
      return focusPanel(goes, state.cursor);
    }
    if (isPressed("deck_timeline", event)) {
      event.preventDefault();
      setView("timeline");
      return focusSoon("stage");
    }
    const step = pressedSlot("move_focus", event);
    const arrow = ["ArrowDown", "ArrowUp", "ArrowLeft", "ArrowRight"].includes(event.key);
    if (step !== -1 || arrow) {
      // The stage's page has its own arrows. Outside the deck they are not ours.
      if (panel === undefined && target !== document.body && target.closest(".deck") === null) return;
      // With nothing focused, left and right are not ours.
      if (panel === undefined && (event.key === "ArrowLeft" || event.key === "ArrowRight")) return;
      if (panel?.id === "stage" && state.sel !== undefined) return;
      event.preventDefault();
      const letter = step !== -1;
      const down = step === 0 || step === 2 || event.key === "ArrowDown";
      if (panel !== undefined && panel.id !== "list") return letter ? stepBy(panel, target, down ? 1 : -1) : stepIn(panel, target, event.key);
      if (event.key === "ArrowLeft" || event.key === "ArrowRight") return;
      const at = listed.findIndex((one) => one.key === state.cursor);
      const next = listed[Math.min(listed.length - 1, Math.max(0, at < 0 ? (down ? 0 : listed.length - 1) : at + (down ? 1 : -1)))];
      if (next !== undefined) rowFocus(next.key);
      return;
    }
    if (isPressed("open_focused", event)) {
      // Pressed on a button of the deck: the press, taken here so a walk playing does not also step on it.
      if (target.tagName === "BUTTON" || target.tagName === "A") {
        if (target.closest(".deck") === null) return;
        event.preventDefault();
        return target.click();
      }
      if (panel !== undefined && panel.id !== "list") {
        // Pressed on the panel itself rather than on a thing in it: the thing focus is on.
        const item = document.activeElement instanceof HTMLElement && panel.items !== undefined ? document.activeElement.closest<HTMLElement>(panel.items) : null;
        if (item === null) return;
        event.preventDefault();
        return item.click();
      }
      const row = listed.find((one) => one.key === state.cursor);
      if (row === undefined) return;
      event.preventDefault();
      return select(selOfSubject(row));
    }
    if (isPressed("close", event)) {
      if (state.sel?.kind === "autopilot") {
        event.preventDefault();
        leaveAutopilot();
      } else if (state.sel !== undefined) {
        event.preventDefault();
        goBack();
        // What had focus may have been what was just put away: the list has it then.
        window.requestAnimationFrame(() => panelAt(document.activeElement) === undefined && focusPanel("list", deckNow().cursor));
      } else if ((panel !== undefined && panel.id !== "list") || target === document.body) {
        event.preventDefault();
        focusPanel("list", state.cursor);
      }
    }
  });

  return <DeckKeySheet open={keySheet} onClose={() => setKeySheet(false)} />;
}
