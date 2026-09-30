// Plan's task panel resting width, remembered across a restart — the owner's
// note of 30 Sep 2026, that it should resize with Helm's own handle. Helm's
// dock width (`@armada/shell`, `dock-width.ts`) is the precedent: one
// `localStorage` key, this window's own layout rather than a Fleet preference.
//
// **Stored as dragged, and clamped where it draws.** Helm's is clamped here
// against `window.innerWidth`; the panel's ceiling is the Job screen's width,
// which only the docked `Sheet` measures, so the clamp is its.

import { useState } from "react";

const KEY = "armada.bridge.task-width";

function read(): number | undefined {
  try {
    const raw = window.localStorage.getItem(KEY);
    if (raw === null) return undefined;
    const value = Number(raw);
    return Number.isFinite(value) ? value : undefined;
  } catch {
    return undefined;
  }
}

function write(width: number): void {
  try {
    window.localStorage.setItem(KEY, String(width));
  } catch {
    // A failed write leaves the width unremembered, the honest answer for a preference.
  }
}

/** The panel's width in px, or absent for `--w-dock` where nothing was dragged yet. */
export function useTaskWidth(): [number | undefined, (width: number) => void] {
  const [width, setWidth] = useState(read);

  function resize(next: number): void {
    setWidth(next);
    write(next);
  }

  return [width, resize];
}
