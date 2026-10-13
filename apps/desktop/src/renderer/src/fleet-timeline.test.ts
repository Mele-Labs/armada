import { describe, expect, it } from "vitest";

import type { TimelineAsk, TimelineEvent } from "@armada/protocol";
import type { BridgeState } from "../../shared/bridge";
import { createFleetTimeline, TIMELINE_WINDOW_MS } from "./fleet-timeline";

const entry = (id: number, says = "Started"): TimelineEvent => ({
  id,
  at: "2026-10-13T10:00:00Z",
  kind: "job.started",
  subject: { kind: "job", id: "job-1" },
  tone: "run",
  says,
  live: true,
});

const NOW = Date.parse("2026-10-13T10:30:00Z");
const settle = () => new Promise((resolve) => setTimeout(resolve, 0));

function fleet(window: TimelineEvent[]) {
  let connection: (facts: Pick<BridgeState, "connection">) => void = () => {};
  let appended: (event: TimelineEvent) => void = () => {};
  const asks: (TimelineAsk | undefined)[] = [];
  let listening = 0;
  return {
    asks,
    listening: () => listening,
    connect: () => connection({ connection: { state: "connected" } as BridgeState["connection"] }),
    append: (event: TimelineEvent) => appended(event),
    api: {
      state: async () => ({ connection: { state: "connecting" } as BridgeState["connection"] }),
      subscribe: (on: typeof connection) => ((connection = on), () => {}),
      getFleetTimeline: async (ask?: TimelineAsk) => (asks.push(ask), { ok: true as const, value: { events: window } }),
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
    expect(held.asks).toEqual([{ since: new Date(NOW - TIMELINE_WINDOW_MS).toISOString(), limit: 2000 }]);
    expect(source.get().map((one) => one.id)).toEqual([1, 2]);
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
    expect(heard.at(-1)).toBe(3);
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
    expect(held.asks).toHaveLength(2);
    expect(source.get()).toHaveLength(1);
  });
});
