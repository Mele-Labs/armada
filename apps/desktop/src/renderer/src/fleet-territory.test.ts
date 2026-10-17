// The deck's areas and touches over Fleet: read when Fleet connects, read again after the board
// changes and while an agent has changed something, empty where Fleet answers none or has gone, and
// left as they stand through a read that was refused.

import { describe, expect, it } from "vitest";

import type { RepositoryAreas, SlotTouches } from "@armada/protocol";
import type { BridgeState } from "../../shared/bridge";
import { createFleetTerritory, SETTLE_MS } from "./fleet-territory";

const AREAS: RepositoryAreas = {
  repositories: [{ manifest_id: "armada", root: "/r/armada", head: "abc", areas: [{ dir: "crates", files: 3, kids: [{ name: "fleet", files: 2 }] }], omitted: 0 }],
};
const TOUCHES: SlotTouches = { repositories: [{ manifest_id: "armada", holders: [{ kind: "session", id: "s1", slot: 1, paths: ["crates/fleet/a.rs"], omitted: 0 }] }] };
const NONE_AREAS: RepositoryAreas = { repositories: [] };
const NONE_TOUCHES: SlotTouches = { repositories: [] };

type Facts = Pick<BridgeState, "connection">;
const up = { connection: { state: "connected" } as BridgeState["connection"] };
const down = { connection: { state: "reading" } as BridgeState["connection"] };
type Answer<T> = T | "refused";

function harness(answers: Array<{ areas: Answer<RepositoryAreas>; touches: Answer<SlotTouches> }>) {
  let on: (facts: Facts) => void = () => {};
  let now: Facts = down;
  let reads = 0;
  let pending: { run: () => void; ms: number } | null = null;
  const refused = { ok: false, outcome: { ok: false, why: "not_connected" } } as const;
  const source = createFleetTerritory(
    {
      state: async () => now,
      subscribe: (to) => ((on = to), () => {}),
      getRepositoryAreas: async () => {
        const answer = answers[Math.min(reads, answers.length - 1)]!.areas;
        return answer === "refused" ? refused : { ok: true, value: answer };
      },
      getSlotTouches: async () => {
        const answer = answers[Math.min(reads++, answers.length - 1)]!.touches;
        return answer === "refused" ? refused : { ok: true, value: answer };
      },
    },
    (run, ms) => {
      pending = { run, ms };
      return () => {
        pending = null;
      };
    },
  );
  const settled = async () => {
    for (let turn = 0; turn < 8; turn++) await Promise.resolve();
  };
  return {
    source,
    listen: async (on: () => void = () => {}) => (source.subscribe(on), await settled()),
    reads: () => reads,
    pending: () => pending,
    tell: async (facts: Facts) => ((now = facts), on(facts), await settled()),
    fire: async () => {
      const due = pending;
      pending = null;
      due?.run();
      await settled();
    },
  };
}

const both = (areas: RepositoryAreas, touches: SlotTouches) => ({ areas, touches });

describe("the deck's areas and touches over Fleet", () => {
  it("are empty until Fleet answers, then come from what it says", async () => {
    const fleet = harness([both(AREAS, TOUCHES)]);
    await fleet.listen();
    expect(fleet.source.get().areas("/r/armada")).toEqual([]);
    expect(fleet.source.get().touches.size).toBe(0);

    await fleet.tell(up);

    expect(fleet.reads()).toBe(1);
    expect(fleet.source.get().areas("/r/armada")).toEqual([{ dir: "crates", kids: [["fleet", 2]] }]);
    expect(fleet.source.get().touches.get("session:s1")).toEqual([{ path: "crates/fleet", w: 1 }]);
  });

  it("are empty where Fleet answers none, as a Fleet with no such route does", async () => {
    const fleet = harness([both(NONE_AREAS, NONE_TOUCHES)]);
    await fleet.listen();
    await fleet.tell(up);

    expect(fleet.source.get().areas("/r/armada")).toEqual([]);
    expect(fleet.source.get().touches.size).toBe(0);
    expect(fleet.pending()).toBeNull();
  });

  it("go empty when what Fleet held is gone, and when Fleet itself has", async () => {
    const fleet = harness([both(AREAS, TOUCHES), both(NONE_AREAS, NONE_TOUCHES)]);
    await fleet.listen();
    await fleet.tell(up);
    await fleet.fire();
    expect(fleet.source.get().touches.size).toBe(0);
    expect(fleet.source.get().areas("/r/armada")).toEqual([]);

    const again = harness([both(AREAS, TOUCHES)]);
    await again.listen();
    await again.tell(up);
    await again.tell(down);
    expect(again.source.get().areas("/r/armada")).toEqual([]);
    expect(again.source.get().touches.size).toBe(0);
    expect(again.pending()).toBeNull();
  });

  it("are read again once the board has changed, one pending read however busy it is", async () => {
    const fleet = harness([both(AREAS, NONE_TOUCHES), both(AREAS, TOUCHES)]);
    await fleet.listen();
    await fleet.tell(up);
    await fleet.tell(up);
    await fleet.tell(up);
    expect(fleet.pending()?.ms).toBe(SETTLE_MS);
    expect(fleet.reads()).toBe(1);

    await fleet.fire();

    expect(fleet.reads()).toBe(2);
    expect(fleet.source.get().touches.get("session:s1")).toBeDefined();
  });

  it("go on being read while an agent has changed something, and stop when none has", async () => {
    const fleet = harness([both(AREAS, TOUCHES), both(AREAS, TOUCHES), both(AREAS, NONE_TOUCHES)]);
    await fleet.listen();
    await fleet.tell(up);
    expect(fleet.pending()).not.toBeNull();
    await fleet.fire();
    expect(fleet.source.get().touches.size).toBe(1);
    await fleet.fire();
    expect(fleet.source.get().touches.size).toBe(0);
    expect(fleet.pending()).toBeNull();
  });

  it("keep what stands through a read that was refused, each half on its own", async () => {
    const fleet = harness([both(AREAS, TOUCHES), { areas: "refused", touches: NONE_TOUCHES }]);
    await fleet.listen();
    await fleet.tell(up);
    await fleet.fire();

    expect(fleet.source.get().areas("/r/armada")).toHaveLength(1);
    expect(fleet.source.get().touches.size).toBe(0);
  });

  it("tell a listener only when something it draws changed", async () => {
    const fleet = harness([both(AREAS, TOUCHES), both(AREAS, TOUCHES)]);
    let told = 0;
    await fleet.listen(() => told++);
    await fleet.tell(up);
    await fleet.fire();
    expect(told).toBe(1);
  });
});
