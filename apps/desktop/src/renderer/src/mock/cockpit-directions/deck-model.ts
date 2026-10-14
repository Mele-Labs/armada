// The flight deck's state and its pure readings. What the owner has open in the dynamic area, which of the two
// views it falls back to, the filters, and the calls he has visited or answered. Held in the window and not in a
// component, because the top bar and the Cockpit body are two trees that must agree, and so a body that unmounts
// while a Job's page is open comes back to the exact view. Mock only. proto/cockpit-three-directions.

import { useSyncExternalStore } from "react";
import type { Item } from "../../Dashboard";
import type { Pr, Ship, Subject } from "./ship";
import { orderOf, toneOf } from "./tabs-model";

/** What the dynamic area is showing. Nothing selected is the map or the timeline. */
export type Sel = { kind: "job" | "session" | "slot" | "pr" | "call"; key: string };
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
};

const FRESH: State = { sel: undefined, cursor: undefined, view: "map", only: undefined, needs: false, visited: new Set(), held: new Map() };
let state: State = FRESH;
const listeners = new Set<() => void>();
const subscribe = (on: () => void) => (listeners.add(on), () => void listeners.delete(on));
const set = (change: Partial<State>) => {
  state = { ...state, ...change };
  listeners.forEach((one) => one());
};

export const forgetDeck = () => {
  state = FRESH;
  listeners.forEach((one) => one());
};
export const useDeck = () => useSyncExternalStore(subscribe, () => state);
export const deckNow = () => state;

export const select = (sel: Sel | undefined) => set({ sel, ...(sel === undefined ? {} : { cursor: sel.kind === "job" || sel.kind === "session" ? sel.key : state.cursor }) });
export const moveCursor = (key: string | undefined) => set({ cursor: key });
export const setView = (view: DeckView) => set({ view });
export const toggleOnly = (only: Only) => set({ only: state.only === only ? undefined : only });
export const toggleNeeds = () => set({ needs: !state.needs });

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

/** The calls still to be opened: not answered, not visited. Fleet's order. */
export const unvisitedOf = (calls: readonly Item[], held: ReadonlyMap<string, number>, visited: ReadonlySet<string>): Item[] => calls.filter((one) => !held.has(one.key) && !visited.has(one.key));

// ---- The list ----------------------------------------------------------------------------------------

/** The Jobs and Sessions the list draws, in standing order, narrowed by the three filters. The map and the timeline keep the same ones. */
export function listOf(ship: Ship, deck: Pick<State, "only" | "needs">): Subject[] {
  return orderOf(ship).filter((one) => (deck.only === undefined || one.kind === (deck.only === "jobs" ? "job" : "session")) && (!deck.needs || one.call !== undefined));
}

export { toneOf };

/** The selection of a subject: its own key, which is what the list and the map both carry. */
export const selOfSubject = (subject: Subject): Sel => ({ kind: subject.kind === "job" ? "job" : "session", key: subject.key });
export const selOfPr = (pr: Pr): Sel => ({ kind: "pr", key: pr.key });
export const slotKey = (root: string, slot: number) => `slot:${root}:${slot}`;

// ---- The map -----------------------------------------------------------------------------------------

export type Region = { root: string; name: string; rows: Subject[] };

/** The map: one region for each repository, the Jobs and Sessions of it in the list's order. */
export function regionsOf(ship: Ship, subjects: readonly Subject[]): Region[] {
  const names = new Map(ship.repos.map((one) => [one.root, one.name]));
  return [...new Set(subjects.map((one) => one.root))].map((root) => ({ root, name: names.get(root) ?? root, rows: subjects.filter((one) => one.root === root) }));
}
