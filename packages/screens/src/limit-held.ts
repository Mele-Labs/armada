// Who is held for the plan limit, and until when. A Session or a Job stopped because the plan's limit was
// hit is woken by Fleet when it resets. The wire shape is not settled, so this is the view-model's own seam:
// nothing sets it in the app yet, and the mock does. Keyed by Session id or Job id; the value is the reset, ISO.

import { useSyncExternalStore } from "react";

let held: ReadonlyMap<string, string> = new Map();
const listeners = new Set<() => void>();

export const limitHeld = {
  until: (id: string): string | undefined => held.get(id),
  set(entries: ReadonlyMap<string, string>): void {
    held = entries;
    listeners.forEach((on) => on());
  },
  subscribe(on: () => void): () => void {
    listeners.add(on);
    return () => void listeners.delete(on);
  },
};

/** When the one held for the limit resumes, or `undefined` where it is not held. */
export function useLimitedUntil(id: string): string | undefined {
  return useSyncExternalStore(limitHeld.subscribe, () => limitHeld.until(id));
}

/** The mark's tooltip: "Limit reached · resumes 6:40 PM", in local time. */
export function limitHeldSaid(until: string): string {
  const at = new Date(until);
  if (Number.isNaN(at.getTime())) return "Limit reached";
  return `Limit reached · resumes ${at.toLocaleTimeString("en-US", { hour: "numeric", minute: "2-digit" })}`;
}
