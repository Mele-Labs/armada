// What a retro and the Lessons listing are drawn from — `retro.ts`.
import { describe, expect, it } from "vitest";

import type { Lesson, Retro } from "@armada/protocol";

import {
  DISAGREE_TIP,
  itemsOf,
  kitAgreeOf,
  lessonRowsOf,
  lessonsTabNamed,
  retroKeyOf,
  retroNumberOf,
  retroSubjectOf,
  statusOf,
  underTab,
  type LessonsTab,
} from "./retro";

const AT = "2026-10-02T21:12:00.000Z";
const SESSION = { id: "01K6Q3AB", title: "Retire two guides" };

const RETRO: Retro = {
  session: SESSION,
  state: "written",
  items: [
    { id: "r", who: "fleet", statement: "The gate failed out_of_bounds.", evidence: ["refusal:1", "refusal:9"] },
    { id: "r", who: "owner", statement: "Helm acted with curl.", evidence: ["restart:2", "ask:1"] },
    { id: "r", who: "drone", statement: "It never saw its test pass.", evidence: ["note:1", "tool:1", "correction:1"] },
  ],
  record: {
    refusals: [{ cite: "refusal:1", at: AT, tool: "Edit", tried: "armada.yml", because: "out of bounds" }],
    restarts: [{ cite: "restart:2", at: AT, actor: "helm", via: "http", moved: "restart" }],
    asks: [{ cite: "ask:1", asked_at: AT, about: "Allow pnpm exec vitest?", answer: "Allow" }],
    notes: [{ cite: "note:1", at: AT, said: "I never saw screens_test pass." }],
    failed_tools: [{ cite: "tool:1", at: AT, tool: "Bash", tried: "find /", because: "not permitted" }],
    corrections: [{ cite: "correction:1", said: "Stop." }],
  },
};

const lesson = (over: Partial<Lesson> & Pick<Lesson, "id" | "statement">): Lesson => ({
  state: "open",
  handle: "01K6Q3AB",
  session: SESSION,
  at: AT,
  who: "owner",
  evidence: [],
  ...over,
});

describe("a retro's items", () => {
  it("resolve each cite to the record row it names, and leave out one the record does not hold", () => {
    const [fleet, owner, drone] = itemsOf(RETRO);
    expect(fleet?.cites).toEqual([
      { id: "refusal:1", name: "Edit", mono: true, detail: "armada.yml · out of bounds", at: AT },
    ]);
    expect(owner?.cites).toEqual([
      { id: "restart:2", name: "restart", mono: true, detail: "helm via http", at: AT },
      { id: "ask:1", name: "Allow pnpm exec vitest?", detail: "Allow", at: AT },
    ]);
    expect(drone?.cites).toEqual([
      { id: "note:1", name: "I never saw screens_test pass.", at: AT },
      { id: "tool:1", name: "Bash", mono: true, detail: "find / · not permitted", at: AT },
      { id: "correction:1", name: "Stop." },
    ]);
  });

  it("keep whose way each got in", () => {
    expect(itemsOf(RETRO).map((one) => one.who)).toEqual(["fleet", "owner", "drone"]);
  });

  it("are none where nothing was written", () => {
    expect(itemsOf({ session: SESSION, state: "pending", record: {} })).toEqual([]);
  });
});

describe("where a retro stands", () => {
  it("says nothing of a written one", () => {
    expect(statusOf(RETRO)).toBeUndefined();
  });

  it("names the other three in bare words, with Fleet's reason where it gave one", () => {
    expect(statusOf({ session: SESSION, state: "pending", record: {} })).toBe("Not written yet");
    expect(statusOf({ session: SESSION, state: "failed", why: "the call timed out", record: {} })).toBe(
      "Not written: the call timed out",
    );
    expect(statusOf({ session: SESSION, state: "skipped", why: "nothing happened", record: {} })).toBe(
      "Skipped: nothing happened",
    );
    expect(statusOf({ session: SESSION, state: "skipped", record: {} })).toBe("Skipped");
  });
});

it("lists each retro item against its Session, with the id on hover", () => {
  const lessons: Lesson[] = [
    lesson({ id: "l-a", statement: "A", who: "fleet" }),
    lesson({ id: "l-b", statement: "B", who: "drone" }),
  ];
  expect(lessonRowsOf(lessons)).toEqual([
    {
      id: "l-a",
      sessionId: "01K6Q3AB",
      who: "fleet",
      statement: "A",
      session: "Retire two guides · s-01K6Q3AB",
      sessionExact: "01K6Q3AB",
      at: AT,
    },
    {
      id: "l-b",
      sessionId: "01K6Q3AB",
      who: "drone",
      statement: "B",
      session: "Retire two guides · s-01K6Q3AB",
      sessionExact: "01K6Q3AB",
      at: AT,
    },
  ]);
});

describe("where a retro item's fix lands", () => {
  const landed: Retro = {
    session: SESSION,
    state: "written",
    record: {},
    items: [
      { id: "r", who: "fleet", statement: "A", evidence: [], lands_in: "armada" },
      { id: "r", who: "owner", statement: "B", evidence: [] },
    ],
  };

  it("rides each item, and is left out of one stored before it was written", () => {
    expect(itemsOf(landed).map((one) => one.landsIn)).toEqual(["armada", undefined]);
    expect("landsIn" in (itemsOf(landed)[1] ?? {})).toBe(false);
  });

  it("rides each lesson row the same way", () => {
    const lessons = [lesson({ id: "l", statement: "A", lands_in: "kit" }), lesson({ id: "l", statement: "B" })];
    expect(lessonRowsOf(lessons).map((one) => one.landsIn)).toEqual(["kit", undefined]);
  });

  it("narrows the retro lessons to one place, and keeps a row with none under All alone", () => {
    const rows = lessonRowsOf([
      lesson({ id: "l", statement: "A", lands_in: "armada" }),
      lesson({ id: "l", statement: "B", lands_in: "kit" }),
      lesson({ id: "l", statement: "C", lands_in: "manifest" }),
      lesson({ id: "l", statement: "D" }),
    ]);
    const said = (tab: LessonsTab) => underTab(rows, tab).map((one) => one.statement);
    expect(said("all")).toEqual(["A", "B", "C", "D"]);
    expect(said("armada")).toEqual(["A"]);
    expect(said("kit")).toEqual(["B"]);
    expect(said("manifest")).toEqual(["C"]);
  });

  it("reads a remembered retro lessons tab back, and anything else as All", () => {
    expect(lessonsTabNamed("kit")).toBe("kit");
    expect(lessonsTabNamed("everything")).toBe("all");
    expect(lessonsTabNamed(null)).toBe("all");
  });
});

describe("an item's title, what happened and fix", () => {
  const written: Retro = {
    session: SESSION,
    state: "written",
    record: {},
    items: [
      { id: "i-new", who: "fleet", lands_in: "armada", statement: "S", title: "T", what: "W", fix: "F", evidence: [] },
      { id: "i-old", who: "owner", statement: "Old", evidence: [] },
    ],
  };

  it("ride the sheet's item, and are left out of one that has only a statement", () => {
    const [fresh, old] = itemsOf(written);
    expect(fresh).toMatchObject({ id: "i-new", title: "T", what: "W", fix: "F" });
    expect(old).toMatchObject({ id: "i-old", statement: "Old" });
    expect("title" in (old ?? {})).toBe(false);
    expect("fix" in (old ?? {})).toBe(false);
  });

  it("ride the Lessons row the same way, keyed by the item's own id", () => {
    const rows = lessonRowsOf([
      lesson({ id: "l-new", who: "fleet", statement: "S", title: "T", what: "W", fix: "F" }),
      lesson({ id: "l-old", statement: "Old" }),
    ]);
    expect(rows.map((one) => one.id)).toEqual(["l-new", "l-old"]);
    expect(rows[0]).toMatchObject({ title: "T", what: "W", fix: "F" });
    expect("title" in (rows[1] ?? {})).toBe(false);
  });
});

describe("what Agree and Disagree say they do", () => {
  it("reads Accept on a Kit item, and Update Kit on one that carries a change", () => {
    expect(kitAgreeOf()).toEqual({ label: "Accept", tip: "Saves it under Accepted." });
    expect(kitAgreeOf(false)).toEqual({ label: "Accept", tip: "Saves it under Accepted." });
    expect(kitAgreeOf(true)).toEqual({
      label: "Update Kit",
      tip: "Adds this command to your Kit's allowed commands. You can remove it from the Kit page.",
    });
    expect(DISAGREE_TIP).toBe("Discards it.");
  });
});

describe("which retro an item belongs to", () => {
  it("reads the number after the last -r in an item's id, hyphens in the Session id and all", () => {
    expect(retroNumberOf("s-01J8ZQ4M-r3-12")).toBe(3);
    expect(retroNumberOf("s-r7-abc-r2-1")).toBe(2);
    expect(retroNumberOf("s-01J8ZQ4M-r10-1")).toBe(10);
  });

  it("has none for an id that does not follow the spelling", () => {
    expect(retroNumberOf("01K7LXYZ-2")).toBeUndefined();
    expect(retroNumberOf("01K7L")).toBeUndefined();
  });

  it("subjects an item with its number, and holds two retros of one Session apart", () => {
    expect(retroSubjectOf({ id: "s-01J8-r2-1", session: { id: "s-01J8" } })).toEqual({ kind: "session", id: "s-01J8", n: 2 });
    expect(retroSubjectOf({ id: "unnumbered", session: { id: "s-01J8" } })).toEqual({ kind: "session", id: "s-01J8" });
    expect(retroKeyOf({ kind: "session", id: "s-01J8", n: 2 })).not.toBe(retroKeyOf({ kind: "session", id: "s-01J8", n: 3 }));
  });
});
