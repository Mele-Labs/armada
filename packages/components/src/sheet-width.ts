// Each kind of sheet's width, remembered across opens and a restart — the
// owner's call of 2 Oct 2026: a log panel dragged wide leaves Plan's task
// panel as it was. `design-system.md`, *Every sheet resizes*.
//
// `localStorage`, one key holding a bag keyed by kind: `panel-open.ts`'s bag
// and `dock-width.ts`'s width, this window's layout rather than a Fleet
// preference. Here rather than beside a screen because `Sheet` reads it, so
// no caller holds a width and none can remember one twice. Stored as dragged;
// `Sheet` clamps it to the area it covers today.

import { useSyncExternalStore } from "react";

const KEY = "armada.bridge.sheet-width";

type Stored = Record<string, number>;

// One store for the window: the diff beside Plan's task panel follows that
// panel while it is dragged, so two mounted sheets have to hear each other.
const listeners = new Set<() => void>();

function read(): Stored {
  try {
    const raw = window.localStorage.getItem(KEY);
    if (raw === null) return {};
    const parsed: unknown = JSON.parse(raw);
    return typeof parsed === "object" && parsed !== null ? (parsed as Stored) : {};
  } catch {
    return {};
  }
}

function write(next: Stored): void {
  try {
    window.localStorage.setItem(KEY, JSON.stringify(next));
  } catch {
    // A failed write leaves the width unremembered, the honest answer for a preference.
  }
}

function subscribe(listener: () => void): () => void {
  listeners.add(listener);
  return () => listeners.delete(listener);
}

function widthOf(kind: string | undefined): number | undefined {
  if (kind === undefined) return undefined;
  const value = read()[kind];
  return typeof value === "number" && Number.isFinite(value) ? value : undefined;
}

/** The width in px a kind of sheet was last dragged to, or absent where it never was. */
export function useSheetWidth(kind: string | undefined): number | undefined {
  return useSyncExternalStore(subscribe, () => widthOf(kind));
}

/** Remember `width` for every sheet of `kind`, and redraw the ones open now. */
export function rememberSheetWidth(kind: string, width: number): void {
  write({ ...read(), [kind]: width });
  for (const listener of listeners) listener();
}
