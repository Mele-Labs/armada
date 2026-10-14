// The flight deck's state and its pure readings. What the owner has open in the dynamic area, which of the two
// views it falls back to, the filters, and the calls he has visited or answered. Held in the window and not in a
// component, because the top bar and the Cockpit body are two trees that must agree, and so a body that unmounts
// while a Job's page is open comes back to the exact view. Mock only. proto/cockpit-flight-deck.

import { useSyncExternalStore } from "react";
import type { WorktreeSlot } from "@armada/protocol";
import type { Item } from "../../Dashboard";
import type { Play } from "./deck-field";
import type { Pr, Ship, Subject } from "./ship";
import { orderOf, toneOf } from "./tabs-model";

/** What the dynamic area is showing. Nothing selected is the map or the timeline. */
export type Sel = { kind: "job" | "session" | "slot" | "pr" | "call" | "checks" | "autopilot"; key: string };
export type DeckView = "map" | "timeline";
export type Only = "jobs" | "sessions";

/**
 * A replay: the field as it was at `at`, playing forward through the events to now. Mock only: the real source is Fleet's event
 * timeline with each slot's observed writes over time, and where a drone stands here derives from the mock's touched paths, so
 * a real replay needs touches recorded with their times.
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
  /** What the area held when Autopilot took it over, and comes back to when it is switched off. */
  before: { sel: Sel | undefined; view: DeckView } | undefined;
};

const FRESH: State = { sel: undefined, focus: undefined, replay: undefined, cursor: undefined, view: "map", only: undefined, needs: false, checks: false, visited: new Set(), held: new Map(), here: true, before: undefined };
let state: State = FRESH;
const listeners = new Set<() => void>();
const subscribe = (on: () => void) => (listeners.add(on), () => void listeners.delete(on));
const set = (change: Partial<State>) => {
  state = { ...state, ...change };
  listeners.forEach((one) => one());
};

/** When each call was first seen: 0 for the ones there at the first frame, the instant for a later one. */
const firstSeen = new Map<string, number>();
let settled = false;

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

/** The calls `w` has opened, oldest first, and which one is on show: `w` goes on, Shift+W goes back. */
export const callTrail = { keys: [] as string[], at: -1 };

export const forgetDeck = () => {
  camera.current = undefined;
  callTrail.keys = [];
  callTrail.at = -1;
  scene.seen.clear();
  scene.drones = undefined;
  scene.landed = undefined;
  state = FRESH;
  firstSeen.clear();
  settled = false;
  listeners.forEach((one) => one());
};
export const useDeck = () => useSyncExternalStore(subscribe, () => state);
export const deckNow = () => state;

export const select = (sel: Sel | undefined) => set({ sel, ...(sel === undefined ? {} : { cursor: sel.kind === "job" || sel.kind === "session" ? sel.key : state.cursor }) });
export const moveCursor = (key: string | undefined) => set({ cursor: key });
/** Choosing a view is going back to it: what was open in the area is put away. */
export const setView = (view: DeckView) => set({ view, sel: undefined });
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
export const stopReplay = () => set({ replay: undefined });

export const toggleOnly = (only: Only) => set({ only: state.only === only ? undefined : only });
export const toggleNeeds = () => set({ needs: !state.needs });
/** The Checks tile is a filter and opens the running Checks in the area; pressed again it is put away. */
export const toggleChecks = () => {
  const on = !state.checks;
  set({ checks: on, sel: on ? { kind: "checks", key: "checks" } : state.sel?.kind === "checks" ? undefined : state.sel });
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

/** Notes the calls Fleet carries. Those there at the first frame share one instant; a later one is later. */
export function noteCalls(calls: readonly Item[]): void {
  for (const one of calls) if (!firstSeen.has(one.key)) firstSeen.set(one.key, settled ? Date.now() : 0);
}
/** The first frame is over: a call seen from here on has arrived, and hails. */
export const settle = () => {
  settled = true;
};
export const arrivedLate = (key: string) => (firstSeen.get(key) ?? 0) > 0;

/** The calls still to be opened, the one that arrived last in front: not answered, not visited. */
export function incomingOf(calls: readonly Item[], held: ReadonlyMap<string, number>, visited: ReadonlySet<string>): Item[] {
  return calls
    .filter((one) => !held.has(one.key) && !visited.has(one.key))
    .map((one, at) => ({ one, at, when: firstSeen.get(one.key) ?? 0 }))
    .sort((a, b) => b.when - a.when || a.at - b.at)
    .map(({ one }) => one);
}

/**
 * Opens a call by opening what it is about, as a row of the list does: its Job or Session, else the pull request it names,
 * and the card itself only where there is neither. Opening it is the visit, so it leaves the tray.
 */
export function openCall(ship: Ship, item: Item): void {
  visit(item.key);
  const owner = item.ownedBy;
  const subject = ship.subjects.find((one) => one.kind !== "pr" && one.call === item.key) ?? (owner === undefined ? undefined : ship.subjects.find((one) => one.kind === owner.kind && one.id === owner.id));
  if (subject !== undefined) return select(selOfSubject(subject));
  const number = /^pull:(\d+)$/.exec(item.key)?.[1];
  const pr = number === undefined ? undefined : ship.prs.find((one) => one.number === Number(number));
  select(pr === undefined ? { kind: "call", key: item.key } : selOfPr(pr));
}

/** The call that is about a Job or Session, while it is not answered. */
export const callOfSubject = (subject: Subject, held: ReadonlyMap<string, number>): string | undefined => (subject.call === undefined || held.has(subject.call) ? undefined : subject.call);

// ---- The list ------------------------------------------------------------------------------------------

/** The Jobs and Sessions the list draws, in standing order, narrowed by the three filters. The map and the timeline keep the same ones. */
export function listOf(ship: Ship, deck: Pick<State, "only" | "needs" | "checks" | "held">): Subject[] {
  return orderOf(ship).filter((one) => (deck.only === undefined || one.kind === (deck.only === "jobs" ? "job" : "session")) && (!deck.needs || callOfSubject(one, deck.held) !== undefined) && (!deck.checks || ship.running.has(one.key)));
}

export { toneOf };

/** The selection of a subject: its own key, which is what the list and the map both carry. */
export const selOfSubject = (subject: Subject): Sel => ({ kind: subject.kind === "job" ? "job" : "session", key: subject.key });
export const selOfPr = (pr: Pr): Sel => ({ kind: "pr", key: pr.key });
export const slotKey = (slot: Pick<WorktreeSlot, "manifest_id" | "slot">) => `slot:${slot.manifest_id}/${slot.slot}`;
