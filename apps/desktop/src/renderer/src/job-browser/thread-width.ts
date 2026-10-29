// The Job thread's width beside the Job browser, remembered across openings: the split the browser sheet's handle drags. Absent until
// dragged, so the stylesheet's own width draws. `localStorage` as `guide-list-width.ts` does: a window's layout, not a Fleet preference.

import { useState } from "react";

const KEY = "armada.bridge.job-browser-thread-width";

function read(): number | undefined {
  try {
    const raw = window.localStorage.getItem(KEY);
    const value = raw === null ? Number.NaN : Number(raw);
    return Number.isFinite(value) ? value : undefined;
  } catch {
    return undefined;
  }
}

export function useThreadWidth(): [number | undefined, (width: number) => void] {
  const [width, setWidth] = useState(read);
  return [
    width,
    (next) => {
      setWidth(next);
      try {
        window.localStorage.setItem(KEY, String(next));
      } catch {
        // A failed write leaves the width unremembered, the honest answer for a preference.
      }
    },
  ];
}
