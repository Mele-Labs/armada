// What an opened thinking run says, row by row: words, and what each row added.
import { describe, expect, it } from "vitest";

import type { Turn } from "@armada/protocol";

import type { DroneThought } from "./draft/drone";
import { called } from "./fixtures/build/base";
import { droneTurnsOf } from "./tab-drones-read";

const STEP = "implement";
const TS = "2026-09-22T10:15:00Z";

function unrecognised(seq: number, kind: string): Turn {
  return { ts: TS, seq, step: STEP, by: "drone", saw: { event: "unrecognised", kind } };
}

const tokens = (seq: number) => unrecognised(seq, "system/thinking_tokens");
const reasoned = (seq: number) => unrecognised(seq, "the Drone's reasoning, not carried");

function read(rows: Turn[], thoughts?: [number, DroneThought][]) {
  return droneTurnsOf(rows, () => null, thoughts === undefined ? undefined : new Map(thoughts));
}

describe("a thinking row reads in words", () => {
  it("never as the wire's kind, with or without the draft", () => {
    const [thinking, reasoning] = read([tokens(1), reasoned(2)]);
    expect(thinking).toMatchObject({ thought: { of: "thinking" }, quiet: true });
    expect(thinking?.subject).toBeUndefined();
    expect(reasoning).toMatchObject({ thought: { of: "reasoned" } });
  });

  it("keeps the wire's kind for a row Armada cannot name", () => {
    expect(read([unrecognised(1, "thinking_delta")])[0]).toMatchObject({ subject: "thinking_delta" });
  });
});

describe("each row carries what it added, not the running estimate", () => {
  it("draws the difference within one call", () => {
    const rows = read([tokens(1), tokens(2), tokens(3)], [
      [1, { of: "tokens", estimated: 400 }],
      [2, { of: "tokens", estimated: 1300 }],
      [3, { of: "tokens", estimated: 1800 }],
    ]);
    expect(rows.map((row) => row.thought)).toEqual([
      { of: "thinking", tokens: 400 },
      { of: "thinking", tokens: 900 },
      { of: "thinking", tokens: 500 },
    ]);
  });

  it("starts again after any other row, which ends the call", () => {
    const rows = read([tokens(1), called(STEP, TS, "c1", "Read", "a.ts"), tokens(3)], [
      [1, { of: "tokens", estimated: 400 }],
      [3, { of: "tokens", estimated: 300 }],
    ]);
    expect(rows[2]?.thought).toEqual({ of: "thinking", tokens: 300 });
  });

  it("starts again where the estimate falls, a new call with no row between", () => {
    const rows = read([tokens(1), tokens(2)], [
      [1, { of: "tokens", estimated: 900 }],
      [2, { of: "tokens", estimated: 200 }],
    ]);
    expect(rows[1]?.thought).toEqual({ of: "thinking", tokens: 200 });
  });
});

describe("a reasoned row carries its text, or says it was withheld", () => {
  it("carries the text", () => {
    const [row] = read([reasoned(1)], [[1, { of: "reasoning", text: "Read it first." }]]);
    expect(row?.thought).toEqual({ of: "reasoned", text: "Read it first." });
  });

  it("marks one the vendor withheld", () => {
    const [row] = read([reasoned(1)], [[1, { of: "reasoning", text: null }]]);
    expect(row?.thought).toEqual({ of: "reasoned", withheld: true });
  });
});
