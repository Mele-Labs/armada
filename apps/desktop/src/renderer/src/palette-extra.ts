// Rows a host page adds to the palette's board block. The mock page fills it with its scenario finder; the Electron build
// never does, so it lists none and imports nothing from the mock.

import { useSyncExternalStore } from "react";
import type { LucideIcon } from "lucide-react";

export type PaletteExtra = { id: string; label: string; icon: LucideIcon; run: () => void };

let rows: readonly PaletteExtra[] = [];
const listeners = new Set<() => void>();

/** Replaces the rows, and returns what takes them away again. */
export function setPaletteExtras(next: readonly PaletteExtra[]): () => void {
  rows = next;
  listeners.forEach((one) => one());
  return () => setPaletteExtras([]);
}

export const usePaletteExtras = (): readonly PaletteExtra[] =>
  useSyncExternalStore(
    (listener) => (listeners.add(listener), () => void listeners.delete(listener)),
    () => rows,
  );
