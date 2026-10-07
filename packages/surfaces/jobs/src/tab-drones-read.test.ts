// What each thinking row adds toward its run's one-line total.
import { describe, expect, it } from "vitest";

import type { Turn } from "@armada/protocol";

import { called } from "@armada/screens/src/fixtures/build/base";
import { THINKING_TOKENS } from "./story";
import { droneTurnsOf } from "./tab-drones-read";

const STEP = "implement";
const TS = "2026-09-22T10:15:00Z";
const DRONE = "01M2D5HKQP001DRONE000T5A";

function unrecognised(seq: number, kind: string): Turn {
  return { ts: TS, seq, step: STEP, by: "drone", drone_id: DRONE, saw: { event: "unrecognised", kind } };
}

/** A thinking row as Fleet sends it since 21.4: the call's running estimate. */
function tokens(seq: number, estimated_tokens: number): Turn {
  return { ts: TS, seq, step: STEP, by: "drone", drone_id: DRONE, saw: { event: "thinking", estimated_tokens } };
}

const reasoned = (seq: number) => unrecognised(seq, "the Drone's reasoning, not carried");
const call = (seq: number): Turn => ({ ...called(STEP, TS, `c${seq}`, "Read", "a.ts"), seq, drone_id: DRONE });

function read(rows: Turn[]) {
  return droneTurnsOf(rows, () => null);
}

describe("a thinking row is quiet, and never the wire's kind", () => {
  it("with its estimate", () => {
    const [thinking, reasoning] = read([tokens(1, 400), reasoned(2)]);
    expect(thinking).toMatchObject({ thought: { of: "thinking", tokens: 400 }, quiet: true });
    expect(thinking?.subject).toBeUndefined();
    // Folds into the same run; it carries no text and adds no tokens.
    expect(reasoning).toMatchObject({ quiet: true });
    expect(reasoning?.thought).toBeUndefined();
  });

  it("and where its line carried no figure, which arrives under the old kind", () => {
    const [thinking] = read([unrecognised(1, THINKING_TOKENS)]);
    expect(thinking).toMatchObject({ thought: { of: "thinking" }, quiet: true });
    expect(thinking?.thought?.tokens).toBeUndefined();
    expect(thinking?.subject).toBeUndefined();
  });

  it("keeps the wire's kind for a row Armada cannot name", () => {
    expect(read([unrecognised(1, "thinking_delta")])[0]).toMatchObject({ subject: "thinking_delta" });
  });
});

describe("each row carries what it added, not the running estimate", () => {
  it("draws the difference within one call", () => {
    const rows = read([tokens(1, 400), tokens(2, 1300), tokens(3, 1800)]);
    expect(rows.map((row) => row.thought)).toEqual([
      { of: "thinking", tokens: 400 },
      { of: "thinking", tokens: 900 },
      { of: "thinking", tokens: 500 },
    ]);
  });

  it("starts again after any other row, which ends the call", () => {
    const rows = read([tokens(1, 400), call(2), tokens(3, 300)]);
    expect(rows[2]?.thought).toEqual({ of: "thinking", tokens: 300 });
  });

  it("starts again after the turn's reasoning row", () => {
    const rows = read([tokens(1, 400), reasoned(2), tokens(3, 700)]);
    expect(rows[2]?.thought).toEqual({ of: "thinking", tokens: 700 });
  });

  it("starts again where the estimate falls, a new call with no row between", () => {
    const rows = read([tokens(1, 900), tokens(2, 200)]);
    expect(rows[1]?.thought).toEqual({ of: "thinking", tokens: 200 });
  });
});
