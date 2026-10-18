// The timeline a scenario's `later` adds to: the entries it starts with, and a batch more each time a walk's `later` step lets time pass.
// A batch is built when it is published, so the entries are dated then and a tool call in it is fresh as the walk shows it.

import type { TimelineEvent } from "@armada/protocol";

import type { TimelineSource } from "../fleet-timeline";

export type LaterEvents = { source: TimelineSource; advance: () => void };

export function eventsLater(first: readonly TimelineEvent[], later: readonly (() => readonly TimelineEvent[])[]): LaterEvents {
  let held = first;
  let passed = 0;
  const listeners = new Set<() => void>();
  return {
    source: {
      get: () => held,
      subscribe: (on) => {
        listeners.add(on);
        return () => listeners.delete(on);
      },
    },
    advance: () => {
      const batch = later[passed++]?.();
      if (batch === undefined) return;
      held = [...held, ...batch];
      for (const on of listeners) on();
    },
  };
}
