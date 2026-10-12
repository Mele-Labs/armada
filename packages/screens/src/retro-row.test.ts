import { describe, expect, it } from "vitest";

import type { RetroRow } from "@armada/protocol";
import { retroRowIsNewer, retroRowOf, withThreadRow } from "./retro-row";
import { rowsOfThread } from "./sessions-wire";

const T0 = "2026-10-12T14:00:00Z";
const T1 = "2026-10-12T14:00:05Z";
const T2 = "2026-10-12T14:00:20Z";

const frame = (over: Partial<RetroRow> = {}): RetroRow => ({
  kind: "retro",
  id: "retro-1",
  at: T0,
  stage: "asking_agent",
  stage_since: T0,
  characters: 0,
  items: [],
  ...over,
});

describe("a retro row replaced by id", () => {
  it("takes a later stage", () => {
    const rows = withThreadRow([frame()], frame({ stage: "reading_record", stage_since: T1 }));
    expect(rows).toHaveLength(1);
    expect(rows[0]).toMatchObject({ stage: "reading_record" });
  });

  it("keeps the later frame when an earlier one arrives after it", () => {
    const writing = frame({ stage: "writing", stage_since: T2, characters: 400, items: [{ title: "A" }] });
    const late = frame({ stage: "reading_record", stage_since: T1 });
    expect(withThreadRow([writing], late)).toEqual([writing]);
  });

  it("keeps the frame with more of the answer when two share a stage", () => {
    const more = frame({ stage: "writing", stage_since: T1, characters: 300, items: [{ title: "A" }, { title: "B" }] });
    const fewer = frame({ stage: "writing", stage_since: T1, characters: 120, items: [{ title: "A" }] });
    expect(withThreadRow([more], fewer)).toEqual([more]);
    expect(withThreadRow([fewer], more)).toEqual([more]);
  });

  it("never takes an ended retro back to writing, even on the same instant", () => {
    const written = frame({ stage: "written", stage_since: T2, retro: 3 });
    expect(withThreadRow([written], frame({ stage: "writing", stage_since: T2, characters: 900 }))).toEqual([written]);
    expect(retroRowIsNewer(written, frame({ stage: "failed", stage_since: T2, why: "late" }))).toBe(false);
  });

  it("lets the same frame through twice", () => {
    const one = frame({ stage: "writing", stage_since: T1, characters: 50 });
    expect(retroRowIsNewer(one, { ...one })).toBe(true);
  });

  it("goes through when an instant will not parse", () => {
    const held = frame({ stage: "writing", stage_since: "soon", characters: 900 });
    expect(retroRowIsNewer(held, frame({ stage: "written", stage_since: "later", retro: 1 }))).toBe(true);
  });

  it("appends a second press's row beside the first, and replaces only its own", () => {
    const first = frame({ stage: "written", retro: 1, stage_since: T1 });
    const second = frame({ id: "retro-2", at: T2, stage_since: T2 });
    const rows = withThreadRow([first], second);
    expect(rows.map((one) => one.id)).toEqual(["retro-1", "retro-2"]);
    const next = withThreadRow(rows, { ...second, stage: "reading_record", stage_since: "2026-10-12T14:00:30Z" });
    expect(next.map((one) => (one as RetroRow).stage)).toEqual(["written", "reading_record"]);
  });

  it("leaves other kinds of row to be replaced as before", () => {
    const check = { kind: "check", id: "c", at: T0, name: "a", run: 1, state: "running" as "running" | "passed" } as const;
    expect(withThreadRow([check], { ...check, state: "passed" })).toEqual([{ ...check, state: "passed" }]);
  });
});

describe("a retro row as the thread draws it", () => {
  it("reads the stage's instant as a number and the items as the card takes them", () => {
    const drawn = retroRowOf(
      frame({ stage: "writing", stage_since: T1, characters: 210, items: [{ title: "A reset waited", who: "agent", lands_in: "armada" }, { who: "owner" }] }),
      "14:00:05",
    );
    expect(drawn).toEqual({
      id: "retro-1",
      at: "14:00:05",
      kind: "retro",
      stage: "writing",
      stageSince: Date.parse(T1),
      characters: 210,
      items: [{ title: "A reset waited", who: "agent", landsIn: "armada" }, { who: "owner" }],
    });
  });

  it("leaves off an end of the route it does not know, rather than drawing a word nobody chose", () => {
    const drawn = retroRowOf(frame({ stage: "writing", items: [{ title: "T", who: "robot" as never, lands_in: "moon" as never }] }), "14:00:00");
    expect(drawn.items).toEqual([{ title: "T" }]);
  });

  it("carries the number and Fleet's words", () => {
    expect(retroRowOf(frame({ stage: "written", retro: 4 }), "x")).toMatchObject({ retro: 4 });
    expect(retroRowOf(frame({ stage: "failed", why: "The model did not answer." }), "x")).toMatchObject({ why: "The model did not answer." });
  });

  it("is a row of the thread in order, beside the rest", () => {
    const rows = rowsOfThread(
      "a",
      [
        { kind: "tool", id: "t", at: T0, text: "Read x" },
        frame({ stage: "written", retro: 2 }),
      ],
      () => undefined,
    );
    expect(rows.map((one) => one.kind)).toEqual(["tool", "retro"]);
  });
});
