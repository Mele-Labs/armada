// A sentence the one bar sent as an Autopilot, held until the Cockpit's start form takes it. A store and
// not a prop, because the bar sends it from the title row and the form may not be mounted yet. #2092.

import { useSyncExternalStore } from "react";

export type Seed = { words: string; at: number };

let held: Seed | null = null;
const listeners = new Set<() => void>();
const told = () => listeners.forEach((one) => one());

/** The one bar's Autopilot choice: the start form opens holding these words, and reads them. */
export function seedAutopilot(words: string): void {
  held = { words, at: Date.now() };
  told();
}

/** Taken by the start form, so a second Cockpit does not open it again. */
export function takeSeed(): void {
  held = null;
  told();
}

export function useAutopilotSeed(): Seed | null {
  return useSyncExternalStore(
    (on) => {
      listeners.add(on);
      return () => void listeners.delete(on);
    },
    () => held,
  );
}
