import { describe, expect, it, vi } from "vitest";

import type { TimelineAsk, TimelineEvent } from "@armada/protocol";
import type { BridgeState } from "../../shared/bridge";
import { TICK_MS } from "./cockpit/deck/field";
import { createFleetTimeline, TIMELINE_WINDOW_MS } from "./fleet-timeline";

const entry = (id: number, says = "Started"): TimelineEvent => ({
  id,
  at: "2026-10-13T10:00:00Z",
  kind: "session.started",
  subject: { kind: "session", id: "s-1" },
  tone: "run",
  says,
  live: true,
});

const NOW = Date.parse("2026-10-13T10:30:00Z");
const settle = () => new Promise((resolve) => setTimeout(resolve, 0));

/** `kept` is what main answers with before Fleet does: the picture it kept on disk. */
function fleet(window: TimelineEvent[], kept: TimelineEvent[] = []) {
  let up = false;
  let connection: (facts: Pick<BridgeState, "connection">) => void = () => {};
  let appended: (event: TimelineEvent) => void = () => {};
  const asks: (TimelineAsk | undefined)[] = [];
  let listening = 0;
  return {
    asks,
    listening: () => listening,
    connect: () => ((up = true), connection({ connection: { state: "connected" } as BridgeState["connection"] })),
    append: (event: TimelineEvent) => appended(event),
    api: {
      state: async () => ({ connection: { state: "connecting" } as BridgeState["connection"] }),
      subscribe: (on: typeof connection) => ((connection = on), () => {}),
      getFleetTimeline: async (ask?: TimelineAsk) => {
        asks.push(ask);
        if (up) return { ok: true as const, value: { events: window } };
        return kept.length === 0 ? { ok: false as const, outcome: { ok: false as const, why: "not_connected" as const } } : { ok: true as const, value: { events: kept } };
      },
      onTimelineAppended: (on: typeof appended) => {
        appended = on;
        listening += 1;
        return () => {
          listening -= 1;
        };
      },
    },
  };
}

describe("the Cockpit's timeline over Fleet", () => {
  it("reads the last two hours once Fleet is connected, and holds nothing before", async () => {
    const held = fleet([entry(1), entry(2)]);
    const source = createFleetTimeline(held.api, () => NOW);
    source.subscribe(() => {});
    expect(source.get()).toEqual([]);

    held.connect();
    await settle();
    const ask = { since: new Date(NOW - TIMELINE_WINDOW_MS).toISOString(), limit: 2000 };
    // Once at once, which main answers with what it kept (nothing here), and once Fleet is connected.
    expect(held.asks).toEqual([ask, ask]);
    expect(source.get().map((one) => one.id)).toEqual([1, 2]);
  });

  it("draws what main kept on disk before Fleet answers, and lets Fleet's own read land over it", async () => {
    const held = fleet([entry(2), entry(3)], [entry(1), entry(2)]);
    const source = createFleetTimeline(held.api, () => NOW);
    source.subscribe(() => {});
    await settle();
    expect(source.get().map((one) => one.id)).toEqual([1, 2]);

    held.connect();
    await settle();
    expect(source.get().map((one) => one.id)).toEqual([1, 2, 3]);
  });

  it("adds each entry Fleet appends, and never draws one twice that the read and an append both carry", async () => {
    const held = fleet([entry(1), entry(2)]);
    const source = createFleetTimeline(held.api, () => NOW);
    const heard: number[] = [];
    source.subscribe(() => heard.push(source.get().length));
    held.connect();
    await settle();

    held.append(entry(3, "Scope passed"));
    held.append(entry(2));
    expect(source.get().map((one) => one.id)).toEqual([1, 2, 3]);
    // The readers are told at the end of the tick the read opened.
    await new Promise((resolve) => setTimeout(resolve, TICK_MS + 50));
    expect(heard.at(-1)).toBe(3);
  });

  it("A timeline burst notifies its readers at most four times a second, and the last event always lands", async () => {
    vi.useFakeTimers();
    try {
      const held = fleet([]);
      const source = createFleetTimeline(held.api, () => NOW);
      const told: number[] = [];
      const lastSeen: (number | undefined)[] = [];
      source.subscribe(() => {
        told.push(Date.now());
        lastSeen.push(source.get().at(-1)?.id);
      });
      held.connect();
      await vi.advanceTimersByTimeAsync(0);
      told.length = 0;
      lastSeen.length = 0;

      // Two hundred appends, five milliseconds apart: a second of a busy Fleet.
      for (let id = 1; id <= 200; id += 1) {
        held.append(entry(id));
        await vi.advanceTimersByTimeAsync(5);
      }
      await vi.advanceTimersByTimeAsync(TICK_MS);

      expect(told.length).toBeGreaterThan(1);
      expect(told.length).toBeLessThanOrEqual(5);
      for (let at = 1; at < told.length; at += 1) expect(told[at]! - told[at - 1]!).toBeGreaterThanOrEqual(TICK_MS);
      expect(lastSeen.at(-1)).toBe(200);
      expect(source.get().at(-1)?.id).toBe(200);
    } finally {
      vi.useRealTimers();
    }
  });

  it("keeps an entry that arrives while the read is out, in order", async () => {
    const held = fleet([entry(1), entry(3)]);
    const source = createFleetTimeline(held.api, () => NOW);
    source.subscribe(() => {});
    held.append(entry(2));
    held.connect();
    await settle();
    expect(source.get().map((one) => one.id)).toEqual([1, 2, 3]);
  });

  it("stops listening with the last window to read it, and reads afresh when one returns", async () => {
    const held = fleet([entry(1)]);
    const source = createFleetTimeline(held.api, () => NOW);
    const off = source.subscribe(() => {});
    expect(held.listening()).toBe(1);
    held.connect();
    await settle();
    expect(source.get()).toHaveLength(1);

    off();
    expect(held.listening()).toBe(0);
    expect(source.get()).toEqual([]);

    source.subscribe(() => {});
    held.connect();
    await settle();
    expect(held.asks).toHaveLength(4);
    expect(source.get()).toHaveLength(1);
  });
});
