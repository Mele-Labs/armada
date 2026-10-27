// The rows the palette lists, as it last drew them. The palette builds its rows from the Jobs, pull requests and pages in
// hand, and Settings' table of the rows that hold a word, a key or a pin has to name them, so the palette says what it
// holds each time that changes and the table reads it here. A window with no palette mounted has none, and the table is
// then left out.

import { useSyncExternalStore } from "react";
import type { PaletteKnown } from "@armada/shell";

let held: readonly PaletteKnown[] = [];
const listeners = new Set<() => void>();

export function setKnownRows(rows: readonly PaletteKnown[]): void {
  held = rows;
  listeners.forEach((one) => one());
}

export const knownRows = (): readonly PaletteKnown[] => held;

export function useKnownRows(): readonly PaletteKnown[] {
  return useSyncExternalStore(
    (on) => {
      listeners.add(on);
      return () => void listeners.delete(on);
    },
    knownRows,
  );
}
