import { describe, expect, it } from "vitest";

import type { PlanUsage } from "@armada/protocol";
import type { BridgeState } from "../../shared/bridge";
import { createFleetPlanUsage } from "./fleet-plan-usage";
import { planUsageDrawn } from "./plan-usage";

const USAGE: PlanUsage = {
  windows: [
    { kind: "five_hour", percent_used: 34, resets_at: "2026-10-12T22:40:00Z" },
    { kind: "seven_day", percent_used: 61 },
  ],
  read_at: "2026-10-12T20:26:00Z",
};

describe("planUsageDrawn", () => {
  it("draws each window's percent_used as percentUsed, leaving resetsAt out where there is none", () => {
    expect(planUsageDrawn(USAGE)).toEqual([
      { kind: "five_hour", percentUsed: 34, resetsAt: "2026-10-12T22:40:00Z" },
      { kind: "seven_day", percentUsed: 61 },
    ]);
  });

  it("draws no windows, so the rings load, for an answer no session has reported to", () => {
    expect(planUsageDrawn({ windows: [] })).toEqual([]);
  });

  it("draws no rings for a reading of no windows, an account off a subscription", () => {
    expect(planUsageDrawn({ windows: [], read_at: "2026-10-21T14:59:00Z" })).toBeUndefined();
  });
});

describe("the plan's windows over Fleet", () => {
  function fleet(reading: PlanUsage = USAGE) {
    let connection: (facts: Pick<BridgeState, "connection">) => void = () => {};
    let changed: (usage: PlanUsage) => void = () => {};
    let reads = 0;
    return {
      reads: () => reads,
      connect: () => connection({ connection: { state: "connected" } as BridgeState["connection"] }),
      changed: (usage: PlanUsage) => changed(usage),
      api: {
        state: async () => ({ connection: { state: "connecting" } as BridgeState["connection"] }),
        subscribe: (on: typeof connection) => ((connection = on), () => {}),
        getPlanUsage: async () => ((reads += 1), { ok: true as const, value: reading }),
        onPlanUsageChanged: (on: typeof changed) => ((changed = on), () => {}),
      },
    };
  }

  it("reads at once for the picture main kept, reads again once Fleet is connected, and holds each newer reading Fleet tells it", async () => {
    const held = fleet();
    const source = createFleetPlanUsage(held.api);
    const heard: (PlanUsage | undefined)[] = [];
    source.subscribe(() => heard.push(source.get()));
    expect(source.get()).toBeUndefined();

    await new Promise((resolve) => setTimeout(resolve, 0));
    expect(held.reads()).toBe(1);
    expect(source.get()).toEqual(USAGE);

    held.connect();
    await new Promise((resolve) => setTimeout(resolve, 0));
    expect(held.reads()).toBe(2);
    expect(source.get()).toEqual(USAGE);

    const newer = { windows: [{ kind: "five_hour", percent_used: 86 }] };
    held.changed(newer);
    expect(source.get()).toEqual(newer);
    expect(heard).toHaveLength(3);
  });

  it("holds Fleet's empty answer as no windows once connected, and nothing before, so the title row can tell loading from absent", async () => {
    const held = fleet({ windows: [] });
    const source = createFleetPlanUsage(held.api);
    source.subscribe(() => {});
    expect(source.get()).toBeUndefined();

    held.connect();
    await new Promise((resolve) => setTimeout(resolve, 0));
    const answered = source.get();
    expect(answered).toEqual({ windows: [] });
    expect(planUsageDrawn(answered!)).toEqual([]);
  });
});
