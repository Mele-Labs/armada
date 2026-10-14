// The flight deck's state and its pure readings. What the owner has open in the dynamic area, which of the two
// views it falls back to, the filters, and the calls he has visited or answered. Held in the window and not in a
// component, because the top bar and the Cockpit body are two trees that must agree, and so a body that unmounts
// while a Job's page is open comes back to the exact view. Mock only. proto/cockpit-flight-deck.

import { useSyncExternalStore } from "react";
import type { WorktreeSlot } from "@armada/protocol";
import type { Item } from "../../Dashboard";
import type { Pr, Ship, Subject } from "./ship";
import { orderOf, toneOf } from "./tabs-model";

/** What the dynamic area is showing. Nothing selected is the map or the timeline. */
export type Sel = { kind: "job" | "session" | "slot" | "pr" | "call" | "checks" | "autopilot"; key: string };
export type DeckView = "map" | "timeline";
export type Only = "jobs" | "sessions";

type State = {
  sel: Sel | undefined;
  /** The row the arrow keys are on in the list. Enter opens it. */
  cursor: string | undefined;
  view: DeckView;
  only: Only | undefined;
  needs: boolean;
  /** Calls opened once: they leave Incoming calls, and the Job or Session that holds one keeps its mark. */
  visited: ReadonlySet<string>;
  /** Calls answered, and when: held out until Fleet stops carrying them. */
  held: ReadonlyMap<string, number>;
  /** The Cockpit is the page in front, so the deck's keys are the window's. */
  here: boolean;
};

const FRESH: State = { sel: undefined, cursor: undefined, view: "map", only: undefined, needs: false, visited: new Set(), held: new Map(), here: true };
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

export const forgetDeck = () => {
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
export const toggleOnly = (only: Only) => set({ only: state.only === only ? undefined : only });
export const toggleNeeds = () => set({ needs: !state.needs });
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
export function listOf(ship: Ship, deck: Pick<State, "only" | "needs" | "held">): Subject[] {
  return orderOf(ship).filter((one) => (deck.only === undefined || one.kind === (deck.only === "jobs" ? "job" : "session")) && (!deck.needs || callOfSubject(one, deck.held) !== undefined));
}

export { toneOf };

/** The selection of a subject: its own key, which is what the list and the map both carry. */
export const selOfSubject = (subject: Subject): Sel => ({ kind: subject.kind === "job" ? "job" : "session", key: subject.key });
export const selOfPr = (pr: Pr): Sel => ({ kind: "pr", key: pr.key });
export const slotKey = (slot: Pick<WorktreeSlot, "manifest_id" | "slot">) => `slot:${slot.manifest_id}/${slot.slot}`;
