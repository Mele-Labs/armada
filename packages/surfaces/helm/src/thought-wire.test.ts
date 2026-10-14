// The wire `crates/ipc/src/thought.rs` writes, read as the TypeScript mirror types it: the exact JSON
// each shape comes as, optional fields absent rather than null, and the `sent` row of the Helm Session's thread.
//
// **Held here and not in `packages/protocol`**, which has no test runner and whose sources are the
// hashed wire.

import { describe, expect, it } from "vitest";

import type { ReadThought, SessionRow, ThoughtReading, ThoughtSent, ThoughtTarget } from "@armada/protocol";

/** What Fleet's JSON decodes to, typed as the mirror says. */
const decoded = <T>(json: string): T => JSON.parse(json) as T;

describe("read_thought", () => {
  it("asks with the words and nothing else", () => {
    const body: ReadThought = { text: "retries should back off" };
    expect(JSON.stringify(body)).toBe('{"text":"retries should back off"}');
  });

  it("answers a question with its kind alone", () => {
    const reading = decoded<ThoughtReading>('{"kind":"question"}');
    expect(reading).toEqual({ kind: "question" });
  });

  it("answers a thought nothing is about with no target, the field absent", () => {
    const reading = decoded<ThoughtReading>('{"kind":"thought"}');
    expect(reading.kind).toBe("thought");
    expect("target" in reading).toBe(false);
  });

  it("answers a thought with its target, optional fields present or absent", () => {
    const full = decoded<ThoughtReading>(
      '{"kind":"thought","target":{"kind":"session","id":"01S","title":"Sessions list views","manifest_id":"01M1","slot":3}}',
    );
    expect(full).toEqual({
      kind: "thought",
      target: { kind: "session", id: "01S", title: "Sessions list views", manifest_id: "01M1", slot: 3 },
    });

    const bare = decoded<ThoughtReading>('{"kind":"thought","target":{"kind":"job","id":"01J","title":"Retry charges"}}');
    if (bare.kind !== "thought" || bare.target === undefined) throw new Error("a thought with a target");
    expect(bare.target.kind).toBe("job");
    expect("manifest_id" in bare.target).toBe(false);
    expect("slot" in bare.target).toBe(false);
  });
});

describe("record_thought_sent", () => {
  it("is said with the words and the target, a missing field left out and never null", () => {
    const target: ThoughtTarget = { kind: "job", id: "01J", title: "Retry charges", manifest_id: "01M2" };
    const body: ThoughtSent = { text: "back off longer", target };
    expect(JSON.stringify(body)).toBe(
      '{"text":"back off longer","target":{"kind":"job","id":"01J","title":"Retry charges","manifest_id":"01M2"}}',
    );
  });
});

describe("the sent row of the Helm Session's thread", () => {
  const json =
    '{"kind":"sent","id":"r5","at":"2026-10-12T10:00:00.000Z","text":"the words","target":{"kind":"session","id":"01S","title":"Sessions list views","slot":3}}';

  it("decodes as a row whose target has the optional fields absent", () => {
    const row = decoded<SessionRow>(json);
    if (row.kind !== "sent") throw new Error("a sent row");
    expect(row.target).toEqual({ kind: "session", id: "01S", title: "Sessions list views", slot: 3 });
    expect("manifest_id" in row.target).toBe(false);
    expect("files" in row).toBe(false);
  });
});
