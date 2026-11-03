// The builds the Usage page reads its figures against, as the window holds them. Fleet serves none yet, so a window starts
// with none: the page draws no Builds panel. A mock scenario hands them in (`mock/mount.tsx`).

import { useSyncExternalStore } from "react";
import type { UsageBuilds } from "@armada/components";

type Held = UsageBuilds | "pending" | undefined;

let held: Held;
const hear = new Set<() => void>();

/** Holds the builds the window draws; nothing held draws no Builds panel. */
export function holdBuilds(next: Held): void {
  held = next;
  for (const one of hear) one();
}

export function useBuilds(): Held {
  return useSyncExternalStore(
    (listen) => {
      hear.add(listen);
      return () => void hear.delete(listen);
    },
    () => held,
  );
}
