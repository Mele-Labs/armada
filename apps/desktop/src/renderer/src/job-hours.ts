// The last 48 hours the Usage page draws as lines, as the window holds them. Fleet serves none yet, so a window starts with
// none: the page draws no hour chart. A mock scenario hands them in (`mock/mount.tsx`).

import { useSyncExternalStore } from "react";
import type { UsageHours } from "@armada/components";

type Held = UsageHours | "pending" | undefined;

let held: Held;
const hear = new Set<() => void>();

/** Holds the hours the window draws; nothing held draws no hour chart. */
export function holdHours(next: Held): void {
  held = next;
  for (const one of hear) one();
}

export function useHours(): Held {
  return useSyncExternalStore(
    (listen) => {
      hear.add(listen);
      return () => void hear.delete(listen);
    },
    () => held,
  );
}
