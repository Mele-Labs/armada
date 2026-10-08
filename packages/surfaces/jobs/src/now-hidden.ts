// Whether the Now panel is hidden, kept for the window and read by whoever draws the panel and
// whoever draws the canvas it focuses. A viewer's convenience, so a blocked store only forgets it.

import { useSyncExternalStore } from "react";

const KEY = "armada.job-now.hidden";
const listeners = new Set<() => void>();
let held: boolean | undefined;

function read(): boolean {
  if (held !== undefined) return held;
  try {
    held = localStorage.getItem(KEY) === "1";
  } catch {
    held = false;
  }
  return held;
}

function write(next: boolean): void {
  held = next;
  try {
    localStorage.setItem(KEY, next ? "1" : "0");
  } catch {
    // Not kept: the panel is as it was left until the window closes.
  }
  listeners.forEach((one) => one());
}

export function useNowHidden(): [boolean, (hidden: boolean) => void] {
  const hidden = useSyncExternalStore(
    (listener) => {
      listeners.add(listener);
      return () => listeners.delete(listener);
    },
    read,
    () => false,
  );
  return [hidden, write];
}
