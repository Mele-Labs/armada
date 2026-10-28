// What happened across the Fleet, over Fleet: the last two hours read when the timeline first draws and once
// Fleet is connected, then each `timeline.appended` added as it arrives. Fleet numbers an entry as it writes
// it, so a read and an append that overlap never draw it twice. The mock's Fleet answers neither call, and
// a scenario hands its entries to the window directly.

import type { TimelineAsk, TimelineEvent } from "@armada/protocol";

import type { FleetTimelineRead } from "../../shared/api/timeline";
import type { BridgeState } from "../../shared/bridge";
import { TICK_MS, throttled } from "./cockpit/deck/field";

/** How far back the window reads. The lanes draw the last hour; the feed's earlier blocks reach past it. */
export const TIMELINE_WINDOW_MS = 2 * 60 * 60_000;

/** Entries Bridge keeps in memory, the newest. */
const KEPT = 2000;

/** What the timeline reads: the entries held, oldest first, kept current while something is subscribed. */
export type TimelineSource = {
  get: () => readonly TimelineEvent[];
  subscribe: (on: () => void) => () => void;
};

/** What Bridge reaches Fleet through: `window.armada` in the app, a fake in a test. */
export type FleetTimelineHost = {
  state: () => Promise<Pick<BridgeState, "connection">>;
  subscribe: (on: (state: Pick<BridgeState, "connection">) => void) => () => void;
  getFleetTimeline: (ask?: TimelineAsk) => Promise<FleetTimelineRead>;
  onTimelineAppended: (on: (event: TimelineEvent) => void) => () => void;
};

const NONE: readonly TimelineEvent[] = [];

export function createFleetTimeline(fleet: FleetTimelineHost, now: () => number = Date.now): TimelineSource {
  let held: readonly TimelineEvent[] = NONE;
  let wasConnected = false;
  const listeners = new Set<() => void>();
  let stop: (() => void) | null = null;

  // Every reader redraws on a notice, and a busy Fleet appends dozens a second: they are told no faster than four a second, the first at
  // once and the last always. What `get` returns is current meanwhile.
  const tell = throttled<void>(() => listeners.forEach((on) => on()), TICK_MS, { now: () => Date.now(), later: (run, ms) => void setTimeout(run, ms) });
  const merge = (more: readonly TimelineEvent[]): void => {
    const seen = new Map(held.map((one) => [one.id, one]));
    for (const one of more) seen.set(one.id, one);
    held = [...seen.values()].sort((a, b) => a.id - b.id).slice(-KEPT);
    tell.push();
  };
  const read = (): void => {
    const since = new Date(now() - TIMELINE_WINDOW_MS).toISOString();
    void fleet.getFleetTimeline({ since, limit: KEPT }).then((answer) => {
      if (answer.ok) merge(answer.value.events);
    });
  };
  const connectedIn = (facts: Pick<BridgeState, "connection">): void => {
    const connected = facts.connection.state === "connected";
    if (connected && !wasConnected) read();
    wasConnected = connected;
  };
  const start = (): void => {
    const offState = fleet.subscribe(connectedIn);
    const offAppended = fleet.onTimelineAppended((one) => merge([one]));
    // Read at once whether or not Fleet answers: main answers from the picture it kept on disk until it does.
    void fleet.state().then((facts) => {
      if (facts.connection.state !== "connected") read();
      connectedIn(facts);
    });
    stop = () => {
      offState();
      offAppended();
      wasConnected = false;
      // What was held is a window that ends when the last window stops listening: the next opening reads afresh.
      held = NONE;
    };
  };

  return {
    get: () => held,
    subscribe: (on) => {
      listeners.add(on);
      if (stop === null) start();
      return () => {
        listeners.delete(on);
        if (listeners.size === 0 && stop !== null) {
          stop();
          stop = null;
        }
      };
    },
  };
}
