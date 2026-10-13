// The write clashes over Fleet: read when Fleet connects, read again after the board changes and
// while a clash stands, and never left marking a Fleet that has gone.

import { describe, expect, it } from "vitest";

import type { WriteClash } from "@armada/protocol";
import type { BridgeState } from "../../shared/bridge";
import { createFleetClashes, SETTLE_MS } from "./fleet-clashes";
import { clashingOf } from "./clashes";

const CLASH: WriteClash = {
  one: { kind: "job", id: "j1", label: "Cap the retry backoff" },
  other: { kind: "session", id: "s1", label: "Retry tuning" },
  paths: ["crates/fleet/src/retry.rs"],
  since: "2026-10-13T09:00:00Z",
};

type Facts = Pick<BridgeState, "connection">;
const up = { connection: { state: "connected" } as BridgeState["connection"] };
const down = { connection: { state: "reading" } as BridgeState["connection"] };

function harness(answers: Array<WriteClash[] | "refused">) {
  let on: (facts: Facts) => void = () => {};
  let now: Facts = down;
  let reads = 0;
  let pending: { run: () => void; ms: number } | null = null;
  const source = createFleetClashes(
    {
      state: async () => now,
      subscribe: (to) => ((on = to), () => {}),
      getWriteClashes: async () => {
        const answer = answers[Math.min(reads++, answers.length - 1)]!;
        return answer === "refused" ? { ok: false, outcome: { ok: false, why: "not_connected" } } : { ok: true, value: answer };
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
    for (let turn = 0; turn < 5; turn++) await Promise.resolve();
  };
  return {
    source,
    /** Someone starts listening while Fleet is still down, as a window does as it opens. */
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

describe("the clashes Fleet reports", () => {
  it("are read when Fleet connects, and held for whoever listens", async () => {
    const fleet = harness([[CLASH]]);
    await fleet.listen();
    expect(fleet.source.get()).toEqual([]);
    await fleet.tell(up);
    expect(fleet.reads()).toBe(1);
    expect(fleet.source.get()).toEqual([CLASH]);
  });

  it("mark the Job and the Session of a pair and nothing else", async () => {
    const fleet = harness([[CLASH]]);
    await fleet.listen();
    await fleet.tell(up);
    const of = clashingOf(fleet.source.get());
    expect(of("job", "j1")).toEqual([{ with: CLASH.other, paths: CLASH.paths }]);
    expect(of("session", "s1")).toEqual([{ with: CLASH.one, paths: CLASH.paths }]);
    expect(of("session", "j1")).toEqual([]);
    expect(of("job", "j2")).toEqual([]);
  });

  it("are read again once the board has changed, one pending read however busy it is", async () => {
    const fleet = harness([[], [CLASH]]);
    await fleet.listen();
    await fleet.tell(up);
    await fleet.tell(up);
    await fleet.tell(up);
    expect(fleet.pending()?.ms).toBe(SETTLE_MS);
    expect(fleet.reads()).toBe(1);
    await fleet.fire();
    expect(fleet.reads()).toBe(2);
    expect(fleet.source.get()).toEqual([CLASH]);
  });

  it("go on being read while one stands, so a cleared one goes, and stop when none does", async () => {
    const fleet = harness([[CLASH], [CLASH], []]);
    await fleet.listen();
    await fleet.tell(up);
    expect(fleet.pending()).not.toBeNull();
    await fleet.fire();
    expect(fleet.source.get()).toEqual([CLASH]);
    await fleet.fire();
    expect(fleet.source.get()).toEqual([]);
    expect(fleet.pending()).toBeNull();
  });

  it("keep what stands through a read that was refused", async () => {
    const fleet = harness([[CLASH], "refused"]);
    await fleet.listen();
    await fleet.tell(up);
    await fleet.fire();
    expect(fleet.source.get()).toEqual([CLASH]);
  });

  it("are dropped with the connection, and read again when it returns", async () => {
    const fleet = harness([[CLASH]]);
    await fleet.listen();
    await fleet.tell(up);
    await fleet.tell(down);
    expect(fleet.source.get()).toEqual([]);
    expect(fleet.pending()).toBeNull();
    await fleet.tell(up);
    expect(fleet.reads()).toBe(2);
    expect(fleet.source.get()).toEqual([CLASH]);
  });

  it("tell a listener only when the list changed", async () => {
    const fleet = harness([[CLASH], [CLASH]]);
    let told = 0;
    await fleet.listen(() => told++);
    await fleet.tell(up);
    await fleet.fire();
    expect(told).toBe(1);
  });
});
