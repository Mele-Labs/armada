// What the flight deck mounts at the window, outside the Cockpit's body: the places in the title bar, and the keys, which are the
// window's wherever the person is (the page chord and the next call work over a page opened from the deck too). **Mounted only while the preview
// switch is on**, so with it off none of this is in the window. The body is `Deck.tsx`; the state is `store.ts`, a module, so
// this, the body and the keys are one thing and a body that unmounts while a page is open comes back exact. #2301.

import { useEffect, useMemo, useState } from "react";
import { isPressed, pressedSlot, useNow } from "@armada/components";
import { DECK_TABS, SURFACE, jobIdOf, useLayout } from "@armada/shell";
import { holdsText } from "@armada/screens/src/keys";

import { useItems, type Hosts } from "../../Dashboard";
import { pickRepository } from "../../commands";
import { useQueued } from "../../sessions-draft";
import type { BridgeState } from "../../../../shared/bridge";
import type { RepositorySummary } from "@armada/protocol";
import { bandingOf } from "./bands";
import { AGE_MS } from "./clock";
import { chordTileFor, flightTileFor, pauseForP, showTile } from "./flight-keys";
import { holdFor } from "./props";
import { walkOrder } from "./queued";
import { DeckKeySheet, armChord, cycleFrom, focusPanel, panelAt, remember, stepBy, stepIn, useDeckKeys } from "./keys";
import { OPENS, PAGE_LETTERS, panelOfSlot, type PageLetter } from "./keys-model";
import { claimsReplay } from "./replay";
import { neighbourOf } from "./reorder";
import { opensFor, placesOff } from "./system-page";
import { TabBar } from "./TabBar";
import { toggleLayer } from "../../session-view/layers";
import { callTrail, deckNow, goBack, incomingOf, leaveAutopilot, needingOf, moveCursor, moveRow, openCall, openSubject, release, select, selOfSubject, setFocus, setHere, setSearching, setView, showRows, useDeck } from "./store";
import { useListed } from "./listed";
import { useShip } from "./ship";
import { useOpenSessionInCockpitAsked } from "../../open-session";

const NO_HOSTS: Hosts = { onOpenSession: () => {}, onOpenLink: () => {} };

/** How long a Session asked for on the deck is waited on before it is let go: one the ship never holds, Helm's own, is the dock's. */
const AWAIT_SESSION_MS = 3000;

type Where = {
  state: BridgeState;
  picked: RepositorySummary | null;
};

const NONE: ReadonlySet<string> = new Set();

/**
 * The places, for the title bar's centre. A call waiting while the window is not on the Cockpit pips the Cockpit tab, which the
 * toast does not cover (the toast speaks on the other tabs and this stays on them). **Pressing Cockpit on a page
 * that was opened from the deck goes back**, so the deck returns exactly as it was left; pressing another place from over the
 * Cockpit replaces the layers, so Back from it returns to the Cockpit.
 */
export function DeckPlaces({ state, picked, showing, away, hidden, onGoTo, onBack, onReplaceNext }: Where & {
  showing: string;
  /** Surfaces nothing serves yet, left off the tabs beside what a layout hides. */
  hidden: readonly string[];
  /** A page opened from the deck stands over the Cockpit: the composer, a report. */
  away: boolean;
  onGoTo: (surface: string) => void;
  onBack: () => void;
  onReplaceNext: () => void;
}) {
  const held = useDeck((deck) => deck.held);
  const visited = useDeck((deck) => deck.visited);
  const off = placesOff(hidden, useLayout("rail").all);
  const calls = useItems("command-central", state, picked, NO_HOSTS, NONE);
  const waiting = incomingOf(calls, held, visited).length > 0;
  const go = (surface: string) => {
    if (surface === SURFACE.overview && away) return onBack();
    if (away) onReplaceNext();
    // The System tab opens the page last used there.
    onGoTo(opensFor(surface, off));
  };
  return <TabBar showing={showing} away={away} onGoTo={go} waiting={waiting} hidden={hidden} />;
}

/**
 * The deck's keys and housekeeping, drawn as nothing but the key sheet. Every key is asked of the keymap (`isPressed`,
 * `pressedSlot`) and none is spelled here.
 */
export function DeckHost({ state, picked, here, hidden, onGoTo, onOpenSession }: Where & {
  /** Surfaces nothing serves yet, left off the places beside what a layout hides. */
  hidden: readonly string[];
  /** The Cockpit is what is on screen, with no page over it. */
  here: boolean;
  onGoTo: (surface: string) => void;
  onOpenSession: (sessionId: string) => void;
}) {
  const ship = useShip({ ...NO_HOSTS, state, now: useNow(AGE_MS), picked }, false);
  const held = useDeck((deck) => deck.held);
  const visited = useDeck((deck) => deck.visited);
  const off = placesOff(hidden, useLayout("rail").all);
  useEffect(() => setHere(here), [here]);

  // An answered call is held out until Fleet stops carrying it, or `holdFor` has passed.
  useEffect(() => {
    if (held.size === 0) return;
    const live = new Set(ship.calls.map((one) => one.key));
    const gone = [...held].filter(([key, since]) => !live.has(key) || Date.now() - since >= holdFor.ms).map(([key]) => key);
    if (gone.length > 0) return release(gone);
    const next = Math.min(...held.values()) + holdFor.ms - Date.now();
    const timer = window.setTimeout(() => release([]), Math.max(0, next));
    return () => window.clearTimeout(timer);
  }, [held, ship.calls]);

  // A Session asked for on the deck (a link to it, a note sent to it) opens as its row does, once the ship holds it: a new one arrives a
  // moment after Fleet answers. **One of another repository than the picked one picks that repository first**, and opens once the ship holds
  // it: the ship is built for the picked repository only. There is no page for one the ship never holds, Helm's own.
  const [awaited, setAwaited] = useState<{ id: string; since: number } | undefined>();
  useOpenSessionInCockpitAsked((id) => setAwaited({ id, since: Date.now() }));
  useEffect(() => {
    if (awaited === undefined) return;
    const subject = ship.subjects.find((one) => one.kind === "session" && one.id === awaited.id);
    if (subject !== undefined) {
      setAwaited(undefined);
      select(selOfSubject(subject));
      if (!deckNow().here) onGoTo(SURFACE.overview);
      return;
    }
    // On All the ship holds every Job already, so only a picked repository is moved.
    // The window's own list is the picked repository's, so the Job is looked up in what Fleet served for all of them.
    const manifest = state.sessions.state === "read" ? state.sessions.sessions.find((one) => one.id === awaited.id)?.manifest_id : undefined;
    const root = manifest === undefined ? undefined : state.holds.repositories?.find((one) => jobIdOf(one) === manifest)?.root;
    if (picked !== null && root !== undefined && root !== picked.root) pickRepository(root);
    const timer = window.setTimeout(() => setAwaited(undefined), Math.max(0, awaited.since + AWAIT_SESSION_MS - Date.now()));
    return () => window.clearTimeout(timer);
  }, [awaited, ship.subjects, onGoTo, state.sessions, state.holds.repositories, picked]);

  const incoming = incomingOf(ship.calls, held, visited, needingOf(ship));
  const repositories = state.holds.repositories;
  const modded = ship.subjects.some((one) => one.mod === true);
  const banding = useMemo(() => bandingOf(picked, repositories ?? [], modded), [picked, repositories, modded]);
  const { listed } = useListed(ship, banding);
  const queued = useQueued();
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
    else {
      // A row past the page drawn: the list draws up to it, and then it is focused.
      showRows(walkOrder(listed, banding, queued, picked).indexOf(key));
      window.requestAnimationFrame(() => row()?.focus());
    }
  };

  const go = (place: (typeof PAGE_LETTERS)[PageLetter]) => {
    onGoTo(opensFor(DECK_TABS.find((one) => one.id === place)!.surface, off));
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
    if (openCall(ship, to, { onOpenSession }) && !deckNow().here) onGoTo(SURFACE.overview);
  };

  useDeckKeys((event) => {
    // Where no repository is served the stage is the start menu, whose own keys 1 to 3 are not the tiles'.
    if ((repositories ?? []).length === 0) return;
    const state = deckNow();
    const target = event.target as HTMLElement;
    // ⌥↑ and ⌥↓ move the row under the cursor past its neighbour, and take the modifier the keys below stand down for.
    const shove = event.altKey && !event.metaKey && !event.ctrlKey ? pressedSlot("deck_reorder", event) : -1;
    if (shove !== -1) {
      if (!state.here || holdsText(event.target) || target.closest?.('[role="menu"]') != null || panelAt(target)?.id !== "list") return;
      const key = target.closest?.<HTMLElement>("[data-row]")?.dataset.row ?? state.cursor;
      const over = key === undefined ? undefined : neighbourOf(listed, banding, key, shove === 0 ? -1 : 1);
      if (key === undefined || over === undefined) return;
      event.preventDefault();
      moveRow(ship, key, over, shove === 1);
      // The row is drawn in its new place as a new element, so focus is put back on it.
      window.requestAnimationFrame(() => rowFocus(key));
      return;
    }
    // A chord the owner gave a tile presses it from anywhere on the deck, but in a text box only with ⌘ or ⌃, where ⌥ and ⇧ type.
    if (event.metaKey || event.ctrlKey || event.altKey) {
      const tile = state.here ? chordTileFor(event) : null;
      if (tile === null) return;
      event.preventDefault();
      showTile(tile);
      tile.click();
      return;
    }
    // The flight controls while they are edited take their own keys (`FlightEdit.tsx`).
    if (target.closest?.("[data-flight-editing]") != null) return;
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
    // Two keys: the page in front reads neither (`armChord` takes the second before it can).
    if (isPressed("deck_page", event)) {
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
    // A replay's own keys, taken by it where it is up.
    if (state.replay !== undefined && claimsReplay(event, panel?.id === "stage" ? "map" : panel?.id)) return;
    if (event.key === "Tab" || event.key === "F6") {
      // Tab goes round the panels from any of them but the stage while it holds a page of its own, whose controls it is left to reach; F6 always.
      if (event.key === "Tab" && (panel === undefined || (panel.id === "stage" && state.sel !== undefined))) return;
      event.preventDefault();
      return focusPanel(cycleFrom(panel?.id, event.shiftKey ? -1 : 1), state.cursor);
    }
    // P presses Pause: the row under the cursor's in the list, elsewhere the open Session's while its Pause wears P. Once Pause has another key that
    // key presses it and P is a tile's, if one was given it. Not an act of the registry, where `p` is Pilot's.
    if (event.key.toLowerCase() === "p" && !event.shiftKey) {
      const row = panel?.id === "list" && state.cursor !== undefined ? document.querySelector<HTMLElement>(`.deck-list__item:has(> [data-row="${CSS.escape(state.cursor)}"]) [data-act="row-pause"]`) : null;
      const act = pauseForP(row);
      if (act !== null) {
        event.preventDefault();
        if (act !== row) showTile(act);
        act.click();
        return;
      }
    }
    // A letter the owner gave a tile. The registry's keys are never given (`keyProblem`), so this reads none of the keys above.
    const lettered = /^[a-z]$/i.test(event.key) ? flightTileFor(event) : null;
    if (lettered !== null) {
      event.preventDefault();
      showTile(lettered);
      lettered.click();
      return;
    }
    const slot = pressedSlot("deck_panel", event);
    const goes = slot === -1 ? undefined : panelOfSlot(slot);
    if (goes !== undefined) {
      event.preventDefault();
      // The key for Needs you goes to the first row that needs him; with none, to the first place that holds a call (a merge queue chip,
      // then the title row's Helm or Sleep); with none, to the list where it was.
      if (goes === "needs") {
        const first = [".deck-row[data-needs]", ".deck-row[data-calling]", ".deck-chip[data-calling]", "[data-calling]"].map((one) => document.querySelector<HTMLElement>(one)).find((one) => one !== null);
        // The title row's menu carries its mark on the span round its button.
        const target = first?.matches("button, [tabindex]") === true ? first : first?.querySelector<HTMLElement>("button");
        if (target === undefined || target === null) return focusPanel("list", state.cursor);
        return target.focus();
      }
      return focusPanel(goes, state.cursor);
    }
    if (isPressed("deck_search", event)) {
      event.preventDefault();
      setSearching(true);
      return;
    }
    // The open Session's Field, large over its stage and back: only where its panel is drawn.
    if (state.sel?.kind === "session" && isPressed("deck_field", event) && document.querySelector('[data-area="field"]') !== null) {
      event.preventDefault();
      return toggleLayer(state.sel.key.replace(/^session:/, ""), "field");
    }
    if (isPressed("deck_timeline", event)) {
      event.preventDefault();
      setView("timeline");
      return focusSoon("stage");
    }
    // `m` is the Dashboard's switch between its views, read by the deck as the map.
    if (isPressed("dashboard_view", event)) {
      event.preventDefault();
      setView("map");
      return focusSoon("stage");
    }
    // Zoom is the map's wherever focus is, while the map is what the stage is showing.
    if (["+", "=", "-", "_", "0", "Home"].includes(event.key) && !event.shiftKey && state.sel === undefined && state.view === "map") {
      const field = document.querySelector(".bf");
      if (field !== null && !field.contains(target)) {
        event.preventDefault();
        field.dispatchEvent(new KeyboardEvent("keydown", { key: event.key, bubbles: true, cancelable: true }));
      }
      return;
    }
    const step = pressedSlot("move_focus", event);
    const arrow = ["ArrowDown", "ArrowUp", "ArrowLeft", "ArrowRight"].includes(event.key);
    // A list that moves its own focus with the arrows (Progress's task list) keeps them, as a layer keeps its Esc.
    if (arrow && target.closest("[data-own-arrows]") !== null) return;
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
      // The ideas saved for later sit in their repository's group, ahead of its rows, and the cursor walks them in that order.
      const order = walkOrder(listed, banding, queued, picked);
      const at = order.findIndex((one) => one === state.cursor);
      // Up from the first row opens the Filter menu, with the search focused.
      if (!down && at === 0) {
        setSearching(true);
        return;
      }
      const next = order[Math.min(order.length - 1, Math.max(0, at < 0 ? (down ? 0 : order.length - 1) : at + (down ? 1 : -1)))];
      if (next !== undefined) rowFocus(next);
      return;
    }
    // `o` is Enter's twin where there is a list to open from: the stage's own page keeps its `o`, and a tile or the Autopilot switch is a control, not an item.
    if (isPressed("open_focused", event) || (isPressed("open", event) && panel !== undefined && OPENS.includes(panel.id))) {
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
      return openSubject(ship, row);
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
      } else if (state.focus !== undefined) {
        // A picked event lets go.
        event.preventDefault();
        setFocus(undefined);
      } else if (state.cursor !== undefined) {
        // Then the cursor: no row is picked, and the map holds no drone's card.
        event.preventDefault();
        moveCursor(undefined);
        document.querySelector<HTMLElement>(".deck-list")?.focus();
      } else if ((panel !== undefined && panel.id !== "list") || target === document.body) {
        event.preventDefault();
        focusPanel("list", state.cursor);
      }
    }
  });

  return <DeckKeySheet open={keySheet} onClose={() => setKeySheet(false)} />;
}
