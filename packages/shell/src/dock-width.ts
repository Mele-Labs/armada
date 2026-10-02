// Helm's dock resting width, remembered across a restart. `panel-open.ts`
// (Bridge/1088, `apps/desktop`) is the precedent — one `localStorage` key,
// this window's own layout rather than a Fleet preference — but that file
// cannot be imported here: `useDock` (below, in `Shell.tsx`) is what actually
// assembles the dock's props, and `@armada/shell` cannot depend on
// `apps/desktop`. Same mechanism, read and written locally instead.
//
// One key rather than `panel-open.ts`'s named bag: there is exactly one dock,
// never a roster of them to key by name.

import { useState } from "react";
import { defaultDockWidth } from "@armada/components";

const KEY = "armada.bridge.dock-width";

function read(): number | null {
  try {
    const raw = window.localStorage.getItem(KEY);
    if (raw === null) return null;
    const value = Number(raw);
    return Number.isFinite(value) ? value : null;
  } catch {
    return null;
  }
}

function write(width: number): void {
  try {
    window.localStorage.setItem(KEY, String(width));
  } catch {
    // A failed write leaves the width unremembered, the honest answer for a preference.
  }
}

/**
 * The dock's width in px, backed by `localStorage`, or `--w-dock` where
 * nothing was saved. **Stored as dragged, and clamped where it draws**: the
 * dock clamps it to the window (`clampDockWidth`), and Helm folded into a
 * sheet, which shares it, to the area that sheet covers — which has no
 * `--w-work-min` to keep, so a clamp here would cap the sheet at the dock's.
 */
export function useDockWidth(): [number, (width: number) => void] {
  const [width, setWidth] = useState(() => read() ?? defaultDockWidth());

  function press(next: number): void {
    setWidth(next);
    write(next);
  }

  return [width, press];
}
