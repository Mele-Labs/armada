// What a Job's Thread tab wears while the Job stands in panes: its state, its name, Rename and its menu. The Job's view publishes them
// and draws no head of its own; the tab is the head.

import type { DropdownMenuEntry, SessionState } from "@armada/components";
import { useLayoutEffect, useSyncExternalStore } from "react";

export type JobHead = {
  state: SessionState;
  /** What the state's mark says on hover. */
  said: string;
  /** Absent until the first turn has named it. */
  title?: string;
  onRename?: (title: string) => void;
  onNameForMe?: () => Promise<unknown>;
  menu: {
    entries: readonly DropdownMenuEntry[];
    onSelect: (id: string) => void;
  };
};

const heads = new Map<string, JobHead>();
const listeners = new Set<() => void>();
const tell = () => {
  for (const hear of listeners) hear();
};

/** Holds the Job's head for its tab while the caller is drawn; absent puts it away. */
export function usePublishJobHead(job: string, head: JobHead | undefined): void {
  useLayoutEffect(() => {
    if (head === undefined) return;
    heads.set(job, head);
    tell();
    return () => {
      if (heads.get(job) === head) {
        heads.delete(job);
        tell();
      }
    };
  });
}

export function useJobHead(job: string): JobHead | undefined {
  return useSyncExternalStore(
    (hear) => (listeners.add(hear), () => void listeners.delete(hear)),
    () => heads.get(job),
  );
}
