// The Fleet-backed source of Autopilot's answers, against a fake Fleet: the list read once connected and
// taken whole when Fleet publishes it, the two acts reaching Fleet's routes, and a correction never raced
// by the review that follows it. What the card does with them is the Cockpit's.

import { describe, expect, it } from "vitest";

import type { AutopilotDecided, AutopilotDecisions, Outcome } from "@armada/protocol";

import type { AutopilotDecisionsActed } from "../../shared/api/autopilot-decisions";
import { createFleetAutopilotDecisions, type FleetAutopilotDecisions } from "./fleet-autopilot-decisions";

const row = (id: string, who: string, over: Partial<AutopilotDecided> = {}): AutopilotDecided => ({
  id,
  manifest_id: "armada",
  who,
  kind: "recommended",
  asked: "Which size?",
  chose: "Large",
  because: "It is the one the Job recommended.",
  at: "2026-11-02T03:00:00.000Z",
  ...over,
});
const held = (...to_review: AutopilotDecided[]): AutopilotDecisions => ({ to_review });
const REFUSED: Outcome = { ok: false, why: "not_connected" };
const settled = () => new Promise((done) => setTimeout(done, 0));

/** A Fleet holding a list, and a record of what Bridge asked of it, in order. */
function fake(start: AutopilotDecisions, refuseOverride = false) {
  let kept = start;
  const asked: string[] = [];
  let hear: (decisions: AutopilotDecisions) => void = () => undefined;
  let release: () => void = () => undefined;
  const without = (id: string): AutopilotDecisions => ({ to_review: kept.to_review.filter((one) => one.id !== id) });
  const fleet: FleetAutopilotDecisions = {
    state: async () => ({ connection: { state: "connected" } }) as never,
    subscribe: () => () => undefined,
    getAutopilotDecisions: async () => ({ ok: true, value: kept }),
    overrideAutopilotDecision: async (id, text) => {
      asked.push(`override ${id} ${text}`);
      // Fleet answers a correction after a beat, and marks the row reviewed itself.
      await new Promise<void>((done) => (release = done));
      if (refuseOverride) return { ok: false, outcome: REFUSED } satisfies AutopilotDecisionsActed;
      return { ok: true, value: (kept = without(id)) };
    },
    reviewAutopilotDecision: async (id) => (asked.push(`review ${id}`), { ok: true, value: (kept = without(id)) }),
    onAutopilotDecisionsChanged: (on) => ((hear = on), () => undefined),
  };
  const source = createFleetAutopilotDecisions(fleet);
  return { source, asked, push: (decisions: AutopilotDecisions) => hear(decisions), finish: () => release() };
}

describe("Autopilot's answers, as Fleet carries them", () => {
  it("reads Fleet's list once connected", async () => {
    const { source } = fake(held(row("s21:q1", "Order the lunch", { session_id: "s21" })));
    source.subscribe(() => undefined);
    await settled();
    expect(source.get().map((one) => one.id)).toEqual(["s21:q1"]);
  });

  it("takes the whole list again whenever Fleet publishes it", async () => {
    const { source, push } = fake(held(row("s21:q1", "Order the lunch")));
    source.subscribe(() => undefined);
    await settled();
    push(held(row("s22:q1", "Name the branch")));
    expect(source.get().map((one) => one.id)).toEqual(["s22:q1"]);
  });
});

describe("the acts on an answer", () => {
  it("reviews an answer it agrees with", async () => {
    const { source, asked } = fake(held(row("d1", "Order the lunch"), row("d2", "Shorten the wait")));
    source.subscribe(() => undefined);
    await settled();
    source.review("d1");
    await settled();
    expect(asked).toEqual(["review d1"]);
    expect(source.get().map((one) => one.id)).toEqual(["d2"]);
  });

  it("sends a correction, and holds the review after it until Fleet has answered", async () => {
    const { source, asked, finish } = fake(held(row("d1", "Order the lunch"), row("d2", "Shorten the wait")));
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
    expect(source.get().map((one) => one.id)).toEqual(["d2"]);
  });

  it("keeps an answer whose correction Fleet refused, rather than reviewing it away", async () => {
    const { source, asked, finish } = fake(held(row("d1", "Order the lunch"), row("d2", "Shorten the wait")), true);
    source.subscribe(() => undefined);
    await settled();
    source.override("d2", "Do not retry");
    source.review("d2");
    finish();
    await settled();
    expect(asked).toEqual(["override d2 Do not retry"]);
    expect(source.get().map((one) => one.id)).toEqual(["d1", "d2"]);
  });
});
