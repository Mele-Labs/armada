// The flight deck's state and its pure readings. What the owner has open in the dynamic area, which of the two
// views it falls back to, the filters, and the calls he has visited or answered. Held in the window and not in a
// component, because the top bar and the Cockpit body are two trees that must agree, and so a body that unmounts
// while a Job's page is open comes back to the exact view. The few ways of reading that should outlive a restart (the
// view, how a Job draws its run, the filters) are remembered through `remembered-views.ts`. #2301.

import { useSyncExternalStore } from "react";
import type { Hosts, Item } from "../../Dashboard";
import { DECK_FILTERS_KEY, DECK_JOB_VIEW_KEY, DECK_ONLY_KEY, DECK_VIEW_KEY, readRemembered, writeRemembered } from "../../remembered-views";
import { sessionIdOf } from "../waiting";
import { orderOf } from "./order";
import type { Play } from "./plays";
import type { Pr, Ship, Subject } from "./ship";

/** What the dynamic area is showing. Nothing selected is the map or the timeline. */
export type Sel = { kind: "job" | "session" | "slot" | "pr" | "call" | "autopilot"; key: string };
export type DeckView = "map" | "timeline";
export type Only = "jobs" | "sessions";

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

type State = {
  sel: Sel | undefined;
  /** The feed row picked: the event the map shows what moved for. */
  focus: string | undefined;
  replay: Replay | undefined;
  /** The row the arrow keys are on in the list. Enter opens it. */
  cursor: string | undefined;
  view: DeckView;
  only: Only | undefined;
  needs: boolean;
  /** Narrowed to what has a Check running now. */
  checks: boolean;
  /** Calls opened once: they leave Incoming calls, and the Job or Session that holds one keeps its mark. */
  visited: ReadonlySet<string>;
  /** Calls answered, and when: held out until Fleet stops carrying them. */
  held: ReadonlyMap<string, number>;
  /** The Cockpit is the page in front, so the deck's keys are the window's. */
  here: boolean;
  /** What was open when a holder was opened from it (a bay's history), so that back returns to it and not to the map. */
  from: Sel | undefined;
  /** How a Job's pane draws its run: the mission across the plane, or the canvas. Kept for the window. */
  jobView: "mission" | "canvas";
  /** What the area held when Autopilot took it over, and comes back to when it is switched off. */
  before: { sel: Sel | undefined; view: DeckView } | undefined;
};

const viewNamed = (value: string | null): DeckView => (value === "timeline" ? "timeline" : "map");
const jobViewNamed = (value: string | null): State["jobView"] => (value === "canvas" ? "canvas" : "mission");
const onlyNamed = (value: string | null): Only | "" => (value === "jobs" || value === "sessions" ? value : "");
const filtersNamed = (value: string | null): string => value ?? "";

/** What a new window starts with: the remembered ways of reading over the plain rest. */
function fresh(): State {
  const only = readRemembered(DECK_ONLY_KEY, onlyNamed);
  const filters = readRemembered(DECK_FILTERS_KEY, filtersNamed).split(" ");
  return {
    sel: undefined,
    focus: undefined,
    replay: undefined,
    cursor: undefined,
    view: readRemembered(DECK_VIEW_KEY, viewNamed),
    only: only === "" ? undefined : only,
    needs: filters.includes("needs"),
    checks: filters.includes("checks"),
    visited: new Set(),
    held: new Map(),
    here: true,
    from: undefined,
    jobView: readRemembered(DECK_JOB_VIEW_KEY, jobViewNamed),
    before: undefined,
  };
}
let state: State = fresh();
const listeners = new Set<() => void>();
const subscribe = (on: () => void) => (listeners.add(on), () => void listeners.delete(on));
const set = (change: Partial<State>) => {
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
export const useDeck = () => useSyncExternalStore(subscribe, () => state);
export const deckNow = () => state;

export const select = (sel: Sel | undefined) => set({ sel, from: undefined, ...(sel === undefined ? {} : { cursor: sel.kind === "job" || sel.kind === "session" ? sel.key : state.cursor }) });
/** Opens a thing from what is open, which back returns to: a holder in a bay's history. */
export const openFrom = (sel: Sel) => set({ from: state.sel, sel, cursor: sel.kind === "job" || sel.kind === "session" ? sel.key : state.cursor });
/** Back from what is open: to what it was opened from, else to the map or timeline. */
export const goBack = () => (state.from === undefined ? select(undefined) : set({ sel: state.from, from: undefined }));
export const moveCursor = (key: string | undefined) => set({ cursor: key });
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

export const setJobView = (jobView: State["jobView"]) => {
  writeRemembered(DECK_JOB_VIEW_KEY, jobView);
  set({ jobView });
};
export const toggleJobView = () => setJobView(state.jobView === "mission" ? "canvas" : "mission");
const keepFilters = (filters: Pick<State, "needs" | "checks">) => writeRemembered(DECK_FILTERS_KEY, [filters.needs ? "needs" : "", filters.checks ? "checks" : ""].filter((one) => one !== "").join(" "));
export const toggleOnly = (only: Only) => {
  const next = state.only === only ? undefined : only;
  writeRemembered(DECK_ONLY_KEY, next ?? "");
  set({ only: next });
};
export const toggleNeeds = () => {
  keepFilters({ needs: !state.needs, checks: state.checks });
  set({ needs: !state.needs });
};
/** The Checks tile is a plain filter, as Jobs only and Sessions only are: what is not running a Check dims, and the list and the feed narrow to the rest. */
export const toggleChecks = () => {
  keepFilters({ needs: state.needs, checks: !state.checks });
  set({ checks: !state.checks });
};
export const setHere = (here: boolean) => {
  if (state.here !== here) set({ here });
};

/** The card is open: the call has been seen, so it leaves the tray. */
export function visit(key: string): void {
  if (!state.visited.has(key)) set({ visited: new Set(state.visited).add(key) });
}
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
export function incomingOf(calls: readonly Item[], held: ReadonlyMap<string, number>, visited: ReadonlySet<string>): Item[] {
  return calls
    .filter((one) => !held.has(one.key) && !visited.has(one.key))
    .map((one, at) => ({ one, at, when: arrivedOf(one) }))
    .sort((a, b) => b.when - a.when || a.at - b.at)
    .map(({ one }) => one);
}

/**
 * Opens a call by opening what it is about, as a row of the list does: its Job or Session, else the pull request it names,
 * and the card itself only where there is none of them. Opening it is the visit, so it leaves the tray.
 *
 * **A subject carries one call, and a Session or Job can raise several** (a Session that asks twice, a Job with a question and a
 * failed Check), so the call is matched to its owner by what the Item names: the Session in its key (`session:<id>:<item>`), the
 * Job it was raised for, and `ownedBy` on a pull request's or main's call. A Session or Job the ship does not hold (Helm's own
 * Session, a Job that has just ended) is opened by `hosts`, the way a link to it is, and never by the card.
 */
export function openCall(ship: Ship, item: Item, hosts?: Pick<Hosts, "onOpen" | "onOpenSession">): boolean {
  visit(item.key);
  const owner = item.ownedBy;
  const sessionId = sessionIdOf(item.key);
  const jobId = item.job?.id;
  const held = (kind: "job" | "session", id: string) => ship.subjects.find((one) => one.kind === kind && one.id === id);
  const subject =
    ship.subjects.find((one) => one.kind !== "pr" && one.call === item.key) ??
    (sessionId === undefined ? undefined : held("session", sessionId)) ??
    (jobId === undefined ? undefined : held("job", jobId)) ??
    (owner === undefined ? undefined : held(owner.kind, owner.id));
  if (subject !== undefined) return picked(selOfSubject(subject));
  const named = sessionId !== undefined ? { kind: "session" as const, id: sessionId } : jobId !== undefined ? { kind: "job" as const, id: jobId } : undefined;
  if (named !== undefined && hosts !== undefined) return handed(named, hosts);
  const number = /^pull:(\d+)$/.exec(item.key)?.[1];
  const pr = number === undefined ? undefined : ship.prs.find((one) => one.number === Number(number));
  if (pr !== undefined) return picked(selOfPr(pr));
  if (owner !== undefined && hosts !== undefined) return handed(owner, hosts);
  return picked({ kind: "call", key: item.key });
}

/** Drawn on the deck's stage: the caller may bring the deck up. */
const picked = (sel: Sel): boolean => (select(sel), true);
/** Opened by the window as a link is: the deck's stage is not where it went. */
const handed = (one: { kind: "job" | "session"; id: string }, hosts: Pick<Hosts, "onOpen" | "onOpenSession">): boolean => (one.kind === "session" ? hosts.onOpenSession(one.id) : hosts.onOpen(one.id), false);

/** The call that is about a Job or Session, while it is not answered. */
export const callOfSubject = (subject: Subject, held: ReadonlyMap<string, number>): string | undefined => (subject.call === undefined || held.has(subject.call) ? undefined : subject.call);

// ---- The list ------------------------------------------------------------------------------------------

/** The Jobs and Sessions the list draws, in standing order, narrowed by the three filters. The map and the timeline keep the same ones. */
export function listOf(ship: Ship, deck: Pick<State, "only" | "needs" | "checks" | "held">): Subject[] {
  return orderOf(ship).filter((one) => (deck.only === undefined || one.kind === (deck.only === "jobs" ? "job" : "session")) && (!deck.needs || callOfSubject(one, deck.held) !== undefined) && (!deck.checks || ship.running.has(one.key)));
}

/**
 * What the feed and the stage's timeline keep: the subjects the list draws, and main's own events only while no filter narrows
 * the list. **One predicate for the three**, so the Jobs-only, Sessions-only, Calls and Checks filters narrow the list, the
 * feed and the stage together.
 */
export function keepOf(listed: readonly Subject[], deck: Pick<State, "only" | "needs" | "checks">): (subject: { kind: string; id?: string }) => boolean {
  const shown = new Set(listed.map((one) => one.key));
  const whole = deck.only === undefined && !deck.needs && !deck.checks;
  return (subject) => (subject.kind === "main" ? whole : shown.has(`${subject.kind}:${subject.id}`));
}

/** The selection of a subject: its own key, which is what the list and the map both carry. */
export const selOfSubject = (subject: Subject): Sel => ({ kind: subject.kind === "job" ? "job" : "session", key: subject.key });
export const selOfPr = (pr: Pr): Sel => ({ kind: "pr", key: pr.key });
