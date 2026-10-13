// The Fleet-backed Sleep source, against a fake Fleet: the night mapped from the wire (a decision's
// `session_id` is the Session it opens), the switch and the two acts reaching Fleet's routes, and a
// correction never raced by the review that follows it. What the card does with them is
// `mock/sleep-cockpit.test.tsx`'s.

import { describe, expect, it } from "vitest";

import type { Outcome, SleepState as SleepWire } from "@armada/protocol";

import type { SleepActed } from "../../shared/api/sleep";
import { createFleetSleep, type FleetSleep } from "./fleet-sleep";

const night = (over: Partial<SleepWire> = {}): SleepWire => ({
  on: true,
  decided: [
    { id: "d1", who: "Order the lunch", asked: "Which size?", chose: "Large", session_id: "s21" },
    { id: "d2", who: "Shorten the wait", asked: "Timed out twice", chose: "Retried" },
  ],
  blocked: [],
  landed: [],
  walks: [],
  ...over,
});
const REFUSED: Outcome = { ok: false, why: "not_connected" };
const settled = () => new Promise((done) => setTimeout(done, 0));

/** A Fleet holding a night, and a record of what Bridge asked of it, in order. */
function fake(start: SleepWire, refuseOverride = false) {
  let held = start;
  const asked: string[] = [];
  let hear: (state: SleepWire) => void = () => undefined;
  let release: () => void = () => undefined;
  const fleet: FleetSleep = {
    state: async () => ({ connection: { state: "connected" } }) as never,
    subscribe: () => () => undefined,
    getSleep: async () => ({ ok: true, value: held }),
    setSleep: async (on) => (asked.push(`set ${on}`), { ok: true, value: (held = { ...held, on }) }),
    overrideSleep: async (id, text) => {
      asked.push(`override ${id} ${text}`);
      // Fleet answers a correction after a beat, and marks the row reviewed itself.
      await new Promise<void>((done) => (release = done));
      if (refuseOverride) return { ok: false, outcome: REFUSED } satisfies SleepActed;
      return { ok: true, value: (held = { ...held, decided: held.decided.filter((one) => one.id !== id) }) };
    },
    reviewSleep: async (id) => (asked.push(`review ${id}`), { ok: true, value: (held = { ...held, decided: held.decided.filter((one) => one.id !== id) }) }),
    onSleepChanged: (on) => ((hear = on), () => undefined),
  };
  const source = createFleetSleep(fleet);
  return { source, asked, push: (state: SleepWire) => hear(state), finish: () => release() };
}

describe("the night, as Fleet carries it", () => {
  it("reads Fleet's night once connected, and names the Session a decision was made in", async () => {
    const { source } = fake(night());
    source.subscribe(() => undefined);
    await settled();
    expect(source.get().on).toBe(true);
    expect(source.get().decided[0]).toEqual({ id: "d1", who: "Order the lunch", asked: "Which size?", chose: "Large", sessionId: "s21" });
  });

  it("leaves a Job's decision with no Session, and no key for one", async () => {
    const { source } = fake(night());
    source.subscribe(() => undefined);
    await settled();
    expect(source.get().decided[1]).not.toHaveProperty("sessionId");
    expect(source.get().decided[1]).not.toHaveProperty("session_id");
  });

  it("takes the whole night again whenever Fleet publishes it", async () => {
    const { source, push } = fake(night());
    source.subscribe(() => undefined);
    await settled();
    push(night({ decided: [{ id: "d9", who: "Name the branch", asked: "fix or feat?", chose: "fix", session_id: "s22" }] }));
    expect(source.get().decided.map((one) => [one.id, one.sessionId])).toEqual([["d9", "s22"]]);
  });
});

describe("the switch", () => {
  it("turns Sleep off on Fleet, which is what the first use of Bridge presses", async () => {
    const { source, asked } = fake(night());
    source.subscribe(() => undefined);
    await settled();
    source.toggle();
    await settled();
    expect(asked).toEqual(["set false"]);
    expect(source.get().on).toBe(false);
  });
});

describe("the acts on a decision", () => {
  it("reviews a decision it agrees with", async () => {
    const { source, asked } = fake(night());
    source.subscribe(() => undefined);
    await settled();
    source.review("d1");
    await settled();
    expect(asked).toEqual(["review d1"]);
    expect(source.get().decided.map((one) => one.id)).toEqual(["d2"]);
  });

  it("sends a correction, and holds the review after it until Fleet has answered", async () => {
    const { source, asked, finish } = fake(night());
    source.subscribe(() => undefined);
    await settled();
    source.override("d1", "Medium");
    source.review("d1");
    await settled();
    expect(asked).toEqual(["override d1 Medium"]);
    finish();
    await settled();
    // Fleet marked the row reviewed with the correction, so there is nothing left to review.
    expect(asked).toEqual(["override d1 Medium"]);
    expect(source.get().decided.map((one) => one.id)).toEqual(["d2"]);
  });

  it("keeps a decision whose correction Fleet refused, rather than reviewing it away", async () => {
    const { source, asked, finish } = fake(night(), true);
    source.subscribe(() => undefined);
    await settled();
    source.override("d2", "Do not retry");
    source.review("d2");
    finish();
    await settled();
    expect(asked).toEqual(["override d2 Do not retry"]);
    expect(source.get().decided.map((one) => one.id)).toEqual(["d1", "d2"]);
  });
});
