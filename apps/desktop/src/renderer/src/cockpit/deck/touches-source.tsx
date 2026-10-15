// Where the flight deck's map gets what each slot's holder has changed there. Fleet's `list_slot_touches` is the source;
// nothing handed, nothing is placed by it, and a drone no tool burst places rests at its pad.

import { createContext, useContext, type ReactNode } from "react";

import type { Touch } from "./touches";

const NONE: ReadonlyMap<string, readonly Touch[]> = new Map();
const Held = createContext<ReadonlyMap<string, readonly Touch[]>>(NONE);

/** Hands the window the areas each holder of a slot has changed, by the holder's key. */
export function SlotTouchesFrom({ touches, children }: { touches: ReadonlyMap<string, readonly Touch[]>; children: ReactNode }) {
  return <Held.Provider value={touches}>{children}</Held.Provider>;
}

export const useSlotTouches = (): ReadonlyMap<string, readonly Touch[]> => useContext(Held);
