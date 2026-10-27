// How the Job's story is drawn, and how its Done zone is read. Both are a viewer's choice, not the Job's.
//
// The take is mock only, read from `?take=`: `spine` draws one timeline line with a NOW marker, Done above it and Next below; `stacked` draws three
// panels, Done, Now and Next. Made once he has looked: the one picked stays and the other goes with this function.
// The ledger view is kept per machine: `step` lists each finished group or step with what it made beneath it, `kind` lists Documents, Pull requests,
// Issues and Artifacts, newest first.

import { useCallback, useSyncExternalStore } from "react";

export type StoryTake = "spine" | "stacked";
export type LedgerView = "step" | "kind";

const TAKES: readonly StoryTake[] = ["spine", "stacked"];

export function storyTake(): StoryTake {
  if (typeof window === "undefined") return "spine";
  const asked = new URLSearchParams(window.location.search).get("take");
  return TAKES.find((one) => one === asked) ?? "spine";
}

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
