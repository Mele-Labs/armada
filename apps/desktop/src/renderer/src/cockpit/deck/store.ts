// The flight deck's state and its pure readings. What the owner has open in the dynamic area, which of the two
// views it falls back to, the filters, and the calls he has visited or answered. Held in the window and not in a
// component, because the top bar and the Cockpit body are two trees that must agree, and so a body that unmounts
// while a layer is open comes back to the exact view. The few ways of reading that should outlive a restart (the
// view, the filters) are remembered through `remembered-views.ts`. #2301.

import { useSyncExternalStore } from "react";
import type { Hosts, Item } from "../../Dashboard";
import { DECK_FILTERS_KEY, DECK_ORDER_KEY, DECK_SCOPE_KEY, DECK_VIEW_KEY, readRemembered, writeRemembered } from "../../remembered-views";
import { forgetPlace, leaving } from "../../session-view/place";
import { sessionIdOf } from "../waiting";
import { bandsOf, type Banding } from "./bands";
import { fixedOrderOf, movedIn, orderOf } from "./order";
import type { Play } from "./plays";
import type { Pr, Ship, Subject } from "./ship";

/** What the dynamic area is showing. Nothing selected is the map or the timeline. */
export type Sel = { kind: "session" | "slot" | "pr" | "call" | "autopilot"; key: string };
export type DeckView = "map" | "timeline";
/** What the list holds: the live Sessions, the Sessions Fleet cannot reach, the ones that ended, or every one of them. */
export type Scope = "active" | "quiet" | "ended" | "all";
export const SCOPES: readonly { id: Scope; label: string }[] = [
  { id: "active", label: "Active" },
  { id: "quiet", label: "Quiet" },
  { id: "ended", label: "Ended" },
  { id: "all", label: "All" },
];
/** How many rows of the list are drawn at first, and how many more each time its end is reached. */
export const LIST_PAGE = 50;

/**
 * A replay: the field as it was at `at`, playing forward through the events to now. Its source is the events in the window
 * Bridge holds (`replay.ts`), so it can start no earlier than the first of them.
 */
export type Replay = {
  /** Where it began, and the playhead. Both are instants, in ms. */
  from: number;
  at: number;
  playing: boolean;
  speed: 1 | 4 | 16;
  /** The effects of the events the playhead has passed, the latest few. */
  plays: readonly Play[];
  /** Which start it is, so a replayed effect is a new one. */
  run: number;
};

export type DeckState = {
  sel: Sel | undefined;
  /** The feed row picked: the event the map shows what moved for. */
  focus: string | undefined;
  replay: Replay | undefined;
  /** The row the arrow keys are on in the list. Enter opens it. */
  cursor: string | undefined;
  /** The Session under the pointer or the keys, in the list or by a bay it holds: the other lights too. */
  hover: string | undefined;
  view: DeckView;
  needs: boolean;
  /** Narrowed to what has a Check running now. */
  checks: boolean;
  scope: Scope;
  /** The keys of the live Sessions in the order he placed them: a new one is added at the end, and only a drag or a key moves one. Kept on this machine. */
  order: readonly string[];
  /** The list's Filter menu is open, its search field focused. */
  searching: boolean;
  /** What the list is searched for: a title, an id, a pull request, a branch, a slot. Kept while the window is. */
  query: string;
  /** How many rows of the list are drawn; the list asks for more as its end is reached. */
  limit: number;
  /** Calls opened once: they stop calling, and the Session that holds one keeps its mark. */
  visited: ReadonlySet<string>;
  /** Calls answered, and when: held out until Fleet stops carrying them. */
  held: ReadonlyMap<string, number>;
  /** The Cockpit is the page in front, so the deck's keys are the window's. */
  here: boolean;
  /** What was open when a holder was opened from it (a bay's history), so that back returns to it and not to the map. */
  from: Sel | undefined;
  /** What the area held when Autopilot took it over, and comes back to when it is switched off. */
  before: { sel: Sel | undefined; view: DeckView } | undefined;
  /** A light on the merge queue that a call asked to open its card: the light pins it and lets go of the ask. */
  light?: "checkout";
};

const viewNamed = (value: string | null): DeckView => (value === "timeline" ? "timeline" : "map");
const filtersNamed = (value: string | null): string => value ?? "";
const scopeNamed = (value: string | null): Scope => SCOPES.find((one) => one.id === value)?.id ?? "active";
/** The stored order, or none where it is missing or is not a list of keys. */
const orderNamed = (value: string | null): string => value ?? "[]";
function orderRead(): readonly string[] {
  try {
    const keys: unknown = JSON.parse(readRemembered(DECK_ORDER_KEY, orderNamed));
    return Array.isArray(keys) ? keys.filter((one): one is string => typeof one === "string") : [];
  } catch {
    return [];
  }
}
/** How many keys are kept once the order outgrows it: the live Sessions only, since a key that is not one is no row. */
const ORDER_KEPT = 300;

/** What a new window starts with: the remembered ways of reading over the plain rest. */
function fresh(): DeckState {
  const filters = readRemembered(DECK_FILTERS_KEY, filtersNamed).split(" ");
  return {
    sel: undefined,
    focus: undefined,
    replay: undefined,
    cursor: undefined,
    hover: undefined,
    view: readRemembered(DECK_VIEW_KEY, viewNamed),
    needs: filters.includes("needs"),
    checks: filters.includes("checks"),
    scope: readRemembered(DECK_SCOPE_KEY, scopeNamed),
    order: orderRead(),
    searching: false,
    query: "",
    limit: LIST_PAGE,
    visited: new Set(),
    held: new Map(),
    here: true,
    from: undefined,
    before: undefined,
  };
}
let state: DeckState = fresh();
const listeners = new Set<() => void>();
const subscribe = (on: () => void) => (listeners.add(on), () => void listeners.delete(on));
const set = (change: Partial<DeckState>) => {
  state = { ...state, ...change };
  listeners.forEach((one) => one());
};

/**
 * What the field remembers between its mounts, so a play is played once: the events
 * and arrivals already shown, and the drones and landed pull requests there were.
 */
export const scene = {
  seen: new Set<string>(),
  drones: undefined as Set<string> | undefined,
  landed: undefined as Set<string> | undefined,
};

/** Where the map's camera was left, so a pane opened over it and closed finds the map as it was. Not state: nothing draws from it. */
export type Camera = { x: number; y: number; eye: number; tilt: boolean };
export const camera = { current: undefined as Camera | undefined };

/** A drone the camera is asked to ease to when the map is next drawn: `w` names the one it goes to. Not state: nothing draws from it. */
export const aimAt = { key: undefined as string | undefined };

/** The calls `w` has opened, oldest first, and which one is on show: `w` goes on, Shift+W goes back. */
export const callTrail = { keys: [] as string[], at: -1 };

export const forgetDeck = () => {
  camera.current = undefined;
  aimAt.key = undefined;
  callTrail.keys = [];
  callTrail.at = -1;
  scene.seen.clear();
  scene.drones = undefined;
  scene.landed = undefined;
  state = fresh();
  listeners.forEach((one) => one());
};
/**
 * The deck's state, whole, or one reading of it: a component that reads one field is drawn when that field changes and not when
 * another does (the cursor moves on every `j`). **A reading must return what it was handed or a primitive**, never a new object,
 * which `useSyncExternalStore` would take as a change on every look.
 */
export function useDeck(): DeckState;
export function useDeck<T>(read: (deck: DeckState) => T): T;
export function useDeck<T>(read?: (deck: DeckState) => T): DeckState | T {
  return useSyncExternalStore(subscribe, () => (read === undefined ? state : read(state)));
}
export const deckNow = () => state;

/** Lights a Session, by its list key, in the list and in the bays. */
export const setHover = (hover: string | undefined) => state.hover !== hover && set({ hover });

export const select = (sel: Sel | undefined) => {
  forgetPlace();
  set({ sel, from: undefined, ...(sel === undefined ? {} : { cursor: sel.kind === "session" ? sel.key : state.cursor }) });
};
/** Opens a thing from what is open, which back returns to: a holder in a bay's history. */
export const openFrom = (sel: Sel) => set({ from: state.sel, sel, cursor: sel.kind === "session" ? sel.key : state.cursor });
/**
 * Opens a Session linked to the one open, from it or to it, which back returns to **as he left it**: the thread where it was scrolled
 * and the Plan groups that were open.
 */
export const openLinked = (sel: Sel) => {
  const was = state.sel;
  if (was?.kind === "session") leaving(was.key.slice("session:".length));
  openFrom(sel);
};
/** Whether one subject is a Session a plan group of the other was handed to, either way round. */
const linkedPair = (a: Subject | undefined, b: Subject): boolean => a !== undefined && (a.linked?.parent === b.key || b.linked?.parent === a.key);
/** Opens a row of the list as a press on it does: a Session linked to the one open is opened from it, so back returns there. */
export const openSubject = (ship: Pick<Ship, "subjects">, subject: Subject): void => {
  const open = state.sel === undefined ? undefined : ship.subjects.find((one) => one.key === state.sel?.key && one.kind !== "pr");
  if (linkedPair(open, subject)) openLinked(selOfSubject(subject));
  else select(selOfSubject(subject));
};
/** Back from what is open: to what it was opened from, else to the map or timeline. */
export const goBack = () => (state.from === undefined ? select(undefined) : set({ sel: state.from, from: undefined, ...(state.from.kind === "session" ? { cursor: state.from.key } : {}) }));
export const moveCursor = (key: string | undefined) => {
  if (state.cursor !== key) set({ cursor: key });
};
/** Choosing a view is going back to it: what was open in the area is put away. */
export const setView = (view: DeckView) => {
  writeRemembered(DECK_VIEW_KEY, view);
  set({ view, sel: undefined, from: undefined });
};
/** Autopilot's configuration takes the area, and what was open is kept to come back to. */
export const enterAutopilot = () => {
  if (state.sel?.kind === "autopilot") return;
  set({ before: { sel: state.sel, view: state.view }, sel: { kind: "autopilot", key: "autopilot" } });
};
/** Autopilot is put away: the area is as it was left. */
export const leaveAutopilot = () => {
  const before = state.before;
  set({ sel: before?.sel, view: before?.view ?? state.view, before: undefined });
};

/** Picks an event, or puts it away when it is the one picked. Choosing one brings the map back and the subject's drone in. */
export const setFocus = (id: string | undefined, subject?: string) => set({ focus: state.focus === id ? undefined : id, ...(id === undefined || state.focus === id ? {} : { view: "map" as const, sel: undefined, ...(subject === undefined ? {} : { cursor: subject }) }) });
let runs = 0;
export const startReplay = (from: number) => {
  runs += 1;
  set({ replay: { from, at: from, playing: true, speed: 1, plays: [], run: runs }, view: "map", sel: undefined });
};
export const changeReplay = (change: Partial<Replay>) => {
  if (state.replay !== undefined) set({ replay: { ...state.replay, ...change } });
};
/** Back to Live: plain, with no picked event left over it. */
export const stopReplay = () => set({ replay: undefined, focus: undefined });

const keepFilters = (filters: Pick<DeckState, "needs" | "checks">) => writeRemembered(DECK_FILTERS_KEY, [filters.needs ? "needs" : "", filters.checks ? "checks" : ""].filter((one) => one !== "").join(" "));
export const toggleNeeds = () => {
  keepFilters({ needs: !state.needs, checks: state.checks });
  set({ needs: !state.needs });
};
/** The Checks tile is a plain filter, as Calls is: what is not running a Check dims, and the list and the feed narrow to the rest. */
export const toggleChecks = () => {
  keepFilters({ needs: state.needs, checks: !state.checks });
  set({ checks: !state.checks });
};
const keepOrder = (order: readonly string[], live: ReadonlySet<string>) => {
  const kept = order.length > ORDER_KEPT ? order.filter((one) => live.has(one)) : order;
  writeRemembered(DECK_ORDER_KEY, JSON.stringify(kept));
  set({ order: kept });
};
/** A Session the order has not met stands at the bottom, in the standing order it would have been drawn in, and nothing placed moves. */
export function placeNew(ship: Ship): void {
  const placed = new Set(state.order);
  const live = orderOf(ship);
  const fresh = live.filter((one) => !placed.has(one.key));
  // A Job that took another's place stands where that one stood.
  const took = new Map<string, string>(fresh.flatMap((one) => (one.earlier !== undefined && placed.has(`session:${one.earlier}`) ? [[`session:${one.earlier}`, one.key] as [string, string]] : [])));
  const rest = fresh.filter((one) => ![...took.values()].includes(one.key));
  if (fresh.length > 0) keepOrder([...state.order.map((one) => took.get(one) ?? one), ...rest.map((one) => one.key)], new Set(live.map((one) => one.key)));
}
/** Puts one live Session before another, or after it: a drag, or ⌥↑ and ⌥↓. */
export function moveRow(ship: Ship, key: string, target: string, after: boolean): void {
  const live = fixedOrderOf(ship, state.order).map((one) => one.key);
  keepOrder(movedIn(live, key, target, after), new Set(live));
}
/** A new scope or a new search starts the list from its first page again. */
export const setScope = (scope: Scope) => {
  writeRemembered(DECK_SCOPE_KEY, scope);
  set({ scope, limit: LIST_PAGE });
};
export const setQuery = (query: string) => set({ query, limit: LIST_PAGE });
export const setSearching = (searching: boolean) => state.searching !== searching && set({ searching });
/** Draws the rows up to `row` (and a page past it), so a key that walks the list is never stopped at its end. */
export const showRows = (row: number) => {
  if (row + 1 > state.limit) set({ limit: Math.ceil((row + 1) / LIST_PAGE) * LIST_PAGE + LIST_PAGE });
};
export const showMoreRows = () => set({ limit: state.limit + LIST_PAGE });
export const setHere = (here: boolean) => {
  if (state.here !== here) set({ here });
};

/** The card is open: the call has been seen, so it leaves the tray. */
export function visit(key: string): void {
  if (!state.visited.has(key)) set({ visited: new Set(state.visited).add(key) });
}
/** The light has pinned the card a call asked it to open. */
export const lightPinned = (): void => set({ light: undefined });
/** The call is answered: held out of every list until Fleet stops carrying it. */
export function answered(key: string): void {
  set({ held: new Map(state.held).set(key, Date.now()), visited: new Set(state.visited).add(key) });
}
export function release(keys: readonly string[]): void {
  if (keys.length > 0) set({ held: new Map([...state.held].filter(([key]) => !keys.includes(key))) });
}

// ---- The calls -----------------------------------------------------------------------------------------

/** When a call arrived, by the Item's own `at`: the instant, or 0 where Fleet gave none. */
export const arrivedOf = (item: Pick<Item, "at">): number => (item.at === undefined ? 0 : Date.parse(item.at) || 0);

/** The calls still to be opened, the one that arrived last in front: not answered, not visited. */
export function incomingOf(calls: readonly Item[], held: ReadonlyMap<string, number>, visited: ReadonlySet<string>, needing: ReadonlySet<string> = new Set()): Item[] {
  const owners = new Set<string>();
  return calls
    // Seen is not enough for a Session that still needs the person: its call stays until it does not.
    .filter((one) => !held.has(one.key) && (!visited.has(one.key) || needing.has(callOwnerOf(one))))
    .map((one, at) => ({ one, at, when: arrivedOf(one) }))
    .sort((a, b) => b.when - a.when || a.at - b.at)
    .map(({ one }) => one)
    .filter((one) => {
      const owner = callOwnerOf(one);
      if (owners.has(owner)) return false;
      owners.add(owner);
      return true;
    });
}

/** The calls without the open Session's own: what it needs of him is a card in its thread, and nothing shows twice. */
export const withoutOpenSession = (calls: readonly Item[], sel: { kind: string; key: string } | undefined): Item[] =>
  sel?.kind === "session" ? calls.filter((one) => callOwnerOf(one) !== sel.key) : [...calls];

/** The Sessions waiting on the person's answer, by the key `callOwnerOf` gives their calls: seeing their call does not end it. */
export const needingOf = (ship: Pick<Ship, "asking">): ReadonlySet<string> => ship.asking;

/**
 * Whom a call is from: its Session, the owner a pull request's or main's call names, else the call alone. **One call each**:
 * a Session with three questions and four items waiting is one call, the newest, since opening it opens the Session that holds them all.
 */
export function callOwnerOf(item: Item): string {
  const session = sessionIdOf(item.key);
  if (session !== undefined) return `session:${session}`;
  if (item.ownedBy !== undefined) return `${item.ownedBy.kind}:${item.ownedBy.id}`;
  return item.key;
}

/**
 * Opens a call by opening what it is about, as a row of the list does: its Session, else the pull request it names,
 * and the card itself only where there is none of them. Opening it is the visit, so it leaves the tray.
 *
 * **A subject carries one call, and a Session can raise several** (a Session that asks twice), so the call is matched to its owner
 * by what the Item names: the Session in its key (`session:<id>:<item>`), and `ownedBy` on a pull request's or main's call. A
 * Session the ship does not hold (Helm's own Session) is opened by `hosts`, the way a link to it is, and never by the card.
 */
export function openCall(ship: Ship, item: Item, hosts?: Pick<Hosts, "onOpenSession">): boolean {
  // The one call stands for every call its owner raised, so opening it is the visit to them all.
  const from = callOwnerOf(item);
  for (const one of ship.calls) if (callOwnerOf(one) === from) visit(one.key);
  visit(item.key);
  const owner = item.ownedBy;
  const sessionId = sessionIdOf(item.key);
  const held = (id: string) => ship.subjects.find((one) => one.kind === "session" && one.id === id);
  const subject =
    ship.subjects.find((one) => one.kind !== "pr" && one.call === item.key) ??
    (sessionId === undefined ? undefined : held(sessionId)) ??
    (owner === undefined ? undefined : held(owner.id));
  if (subject !== undefined) return picked(selOfSubject(subject));
  // A held checkout is the merge queue's light: opening its call pins the light's card and leaves the stage as it was.
  if (item.key.startsWith("checkout:")) return (set({ light: "checkout" }), false);
  if (sessionId !== undefined && hosts !== undefined) return handed(sessionId, hosts);
  const number = /^pull:(\d+)$/.exec(item.key)?.[1];
  const pr = number === undefined ? undefined : ship.prs.find((one) => one.number === Number(number));
  if (pr !== undefined) return picked(selOfPr(pr));
  if (owner !== undefined && hosts !== undefined) return handed(owner.id, hosts);
  return picked({ kind: "call", key: item.key });
}

/** Drawn on the deck's stage: the caller may bring the deck up. */
const picked = (sel: Sel): boolean => (select(sel), true);
/** Opened by the window as a link is: the deck's stage is not where it went. */
const handed = (id: string, hosts: Pick<Hosts, "onOpenSession">): boolean => (hosts.onOpenSession(id), false);

/** The call that is about a Session, while it is not answered. */
export const callOfSubject = (subject: Subject, held: ReadonlyMap<string, number>): string | undefined => (subject.call === undefined || held.has(subject.call) ? undefined : subject.call);

// ---- The list ------------------------------------------------------------------------------------------

/**
 * The Sessions the list draws, narrowed by the three filters and by the search. The map and the timeline keep the same ones.
 * **Active is the live ones in the order he placed them, the new ones last; Quiet and Ended are the Sessions that are not, newest first**, an ended one last seen
 * before the week only where a search is. All is both. The Session open on the stage is in the list wherever it came from.
 */
export function listOf(ship: Ship, deck: Pick<DeckState, "needs" | "checks" | "held"> & { order?: readonly string[]; scope?: Scope; found?: ReadonlyMap<string, unknown>; open?: string }, banding?: readonly Banding[]): Subject[] {
  const scope = deck.scope ?? "active";
  const past = ship.subjects.filter((one) => one.past !== undefined).sort((a, b) => (b.at ?? 0) - (a.at ?? 0));
  const held = past.filter((one) => (scope === "all" || scope === one.past) && (one.older !== true || deck.found !== undefined) && one.behind !== true);
  const live = fixedOrderOf(ship, deck.order ?? []);
  const base = scope === "quiet" || scope === "ended" ? held : scope === "all" ? [...live, ...held] : live;
  const kept = base.filter((one) => (!deck.needs || callOfSubject(one, deck.held) !== undefined) && (!deck.checks || ship.running.has(one.key)) && (deck.found === undefined || deck.found.has(one.key)));
  const open = past.find((one) => one.key === deck.open && !kept.includes(one) && one.behind !== true);
  const rows = linkedUnder(open === undefined ? kept : [...kept, open]);
  // The one order the list draws and the cursor walks: grouped by repository where the list is.
  return banding === undefined ? rows : bandsOf(rows, banding).flatMap((band) => band.subjects);
}

/** Each Session a plan group was handed to stands right under the Session it was handed from, when that one is listed too. */
function linkedUnder(rows: readonly Subject[]): Subject[] {
  const listed = new Set(rows.map((one) => one.key));
  const under = (parent: Subject) => rows.filter((one) => one.linked?.parent === parent.key);
  return rows.filter((one) => one.linked === undefined || !listed.has(one.linked.parent)).flatMap((one) => [one, ...under(one)]);
}

/**
 * What the feed and the stage's timeline keep: the subjects the list draws, and main's own events only while no filter narrows
 * the list. **One predicate for the three**, so the Calls and Checks filters narrow the list, the
 * feed and the stage together.
 */
export function keepOf(listed: readonly Subject[], deck: Pick<DeckState, "needs" | "checks"> & Partial<Pick<DeckState, "scope" | "query">>): (subject: { kind: string; id?: string }) => boolean {
  const shown = new Set(listed.map((one) => one.key));
  const whole = !deck.needs && !deck.checks && (deck.scope ?? "active") === "active" && (deck.query ?? "").trim() === "";
  return (subject) => (subject.kind === "main" ? whole : shown.has(`${subject.kind}:${subject.id}`));
}

/** The selection of a subject: its own key, which is what the list and the map both carry. */
export const selOfSubject = (subject: Subject): Sel => ({ kind: "session", key: subject.key });
export const selOfPr = (pr: Pr): Sel => ({ kind: "pr", key: pr.key });
