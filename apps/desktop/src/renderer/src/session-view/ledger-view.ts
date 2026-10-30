// How Done is read, kept per machine: `step` lists each finished group or step with what it made beneath it, `kind` lists Documents, Pull requests,
// Issues and Artifacts, newest first.

import { useCallback, useSyncExternalStore } from "react";

export type LedgerView = "step" | "kind";

const KEPT = "armada.bridge.job-ledger";
const hear = new Set<() => void>();

function read(): LedgerView {
  try {
    return window.localStorage.getItem(KEPT) === "kind" ? "kind" : "step";
  } catch {
    return "step";
  }
}

/** The view and its setter. A blocked store only forgets it. */
export function useLedgerView(): [LedgerView, (view: LedgerView) => void] {
  const view = useSyncExternalStore(
    (listener) => (hear.add(listener), () => void hear.delete(listener)),
    read,
  );
  const set = useCallback((next: LedgerView) => {
    try {
      window.localStorage.setItem(KEPT, next);
    } catch {
      // Not kept past this window.
    }
    hear.forEach((one) => one());
  }, []);
  return [view, set];
}

/** Flips the view; the key for it (`story.ts`) calls this. */
export function flipLedgerView(): void {
  try {
    window.localStorage.setItem(KEPT, read() === "step" ? "kind" : "step");
  } catch {
    // Not kept past this window.
  }
  hear.forEach((one) => one());
}
