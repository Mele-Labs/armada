// Where Jobs' time went, as the window holds it. Fleet serves no such reading yet, so a window starts with none: the Usage
// page offers no Time view and a Job's System tab draws no Time section. A mock scenario hands one in (`mock/mount.tsx`).

import { useSyncExternalStore } from "react";
import type { UsageTime } from "@armada/components";

type Held = UsageTime | "pending" | undefined;

let held: Held;
const hear = new Set<() => void>();

/** Holds the reading the window draws; nothing held draws no time. */
export function holdJobTime(next: Held): void {
  held = next;
  for (const one of hear) one();
}

export function useJobTime(): Held {
  return useSyncExternalStore(
    (listen) => {
      hear.add(listen);
      return () => void hear.delete(listen);
    },
    () => held,
  );
}
