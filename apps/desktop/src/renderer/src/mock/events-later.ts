// The timeline a scenario's `later` adds to: the entries it starts with, and a batch more each time a walk's `later` step lets time pass.
// A batch is built when it is published, so the entries are dated then and a tool call in it is fresh as the walk shows it. A batch that
// says time `passes` dates every entry held so far that many seconds earlier, so what was fresh can go stale.

import type { TimelineEvent } from "@armada/protocol";

import type { TimelineSource } from "../fleet-timeline";

export type LaterEvents = { source: TimelineSource; advance: () => void };

/** One step's batch: entries added, or time passing over the entries held, and entries added after it. */
export type LaterBatch = readonly TimelineEvent[] | { passes: number; add?: readonly TimelineEvent[] };

const earlier = (one: TimelineEvent, seconds: number): TimelineEvent => ({ ...one, at: new Date(Date.parse(one.at) - seconds * 1000).toISOString() });

export function eventsLater(first: readonly TimelineEvent[], later: readonly (() => LaterBatch)[]): LaterEvents {
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
      held = "passes" in batch ? [...held.map((one) => earlier(one, batch.passes)), ...(batch.add ?? [])] : [...held, ...batch];
      for (const on of listeners) on();
    },
  };
}
