// What a retro and the Lessons listing are drawn from — `retro.ts`.
import { describe, expect, it } from "vitest";

import type { JobRetro, Lesson } from "@armada/protocol";

import { absoluteOf } from "./duration";
import {
  agreeTipOf,
  DISAGREE_TIP,
  itemsOf,
  lessonRowsOf,
  lessonsTabNamed,
  notesOf,
  statusOf,
  underTab,
  type LessonsTab,
} from "./retro";

const AT = "2026-10-02T21:12:00.000Z";

const RETRO: JobRetro = {
  job_id: "01K6Q3JOB3",
  state: "written",
  items: [
    { id: "r", who: "fleet", statement: "The gate failed out_of_bounds.", evidence: ["check:1", "check:9"] },
    { id: "r", who: "owner", statement: "Helm acted with curl.", evidence: ["act:2", "waited:1", "asked:1"] },
    { id: "r", who: "drone", statement: "It never saw its test pass.", evidence: ["said:1", "refusal:1", "not_met:1"] },
  ],
  record: {
    failed_checks: [
      { cite: "check:1", at: AT, step: "implement", name: "out_of_bounds", run: "gate", produced: "armada.yml changed" },
    ],
    acts: [{ cite: "act:2", at: AT, actor: "helm", via: "http", moved: "restart_step" }],
    waited: [{ cite: "waited:1", status: "awaiting_human", from: AT, ms: 1_140_000 }],
    asked: [{ cite: "asked:1", asked_at: AT, about: "Allow pnpm exec vitest?", answer: "Allow" }],
    said_after: [{ cite: "said:1", step: "implement", said: "I never saw screens_test pass." }],
    refusals: [{ cite: "refusal:1", at: AT, tool: "Edit", tried: "armada.yml", because: "out of bounds" }],
    not_met: [
      { cite: "not_met:1", step: "plan", attempt: 1, criterion: "addresses_the_request", produced: "T4 adds docs" },
    ],
  },
  annotations: [{ id: "n1", at: AT, text: "Why twice?" }],
};

describe("a retro's items", () => {
  it("resolve each cite to the record row it names, and leave out one the record does not hold", () => {
    const [fleet, owner, drone] = itemsOf(RETRO);
    expect(fleet?.cites).toEqual([
      { id: "check:1", name: "out_of_bounds", mono: true, detail: "armada.yml changed", when: absoluteOf(AT) },
    ]);
    expect(owner?.cites).toEqual([
      { id: "act:2", name: "restart_step", mono: true, detail: "helm via http", when: absoluteOf(AT) },
      { id: "waited:1", name: "awaiting_human", mono: true, detail: "19m 00s", when: absoluteOf(AT) },
      { id: "asked:1", name: "Allow pnpm exec vitest?", detail: "Allow", when: absoluteOf(AT) },
    ]);
    expect(drone?.cites).toEqual([
      { id: "said:1", name: "implement", mono: true, detail: "I never saw screens_test pass." },
      { id: "refusal:1", name: "Edit", mono: true, detail: "armada.yml · out of bounds", when: absoluteOf(AT) },
      { id: "not_met:1", name: "addresses_the_request", mono: true, detail: "T4 adds docs" },
    ]);
  });

  it("keep whose way each got in", () => {
    expect(itemsOf(RETRO).map((one) => one.who)).toEqual(["fleet", "owner", "drone"]);
  });

  it("are none where nothing was written", () => {
    expect(itemsOf({ job_id: "j", state: "pending", record: {} })).toEqual([]);
  });
});

describe("where a retro stands", () => {
  it("says nothing of a written one", () => {
    expect(statusOf(RETRO)).toBeUndefined();
  });

  it("names the other three in bare words, with Fleet's reason where it gave one", () => {
    expect(statusOf({ job_id: "j", state: "pending", record: {} })).toBe("Not written yet");
    expect(statusOf({ job_id: "j", state: "failed", why: "the call timed out", record: {} })).toBe(
      "Not written: the call timed out",
    );
    expect(statusOf({ job_id: "j", state: "skipped", why: "no Drone ran on it", record: {} })).toBe(
      "Skipped: no Drone ran on it",
    );
    expect(statusOf({ job_id: "j", state: "skipped", record: {} })).toBe("Skipped");
  });
});

it("draws the owner's notes linked to a retro, oldest first as Fleet sends them", () => {
  expect(notesOf(RETRO)).toEqual([{ id: "n1", text: "Why twice?", when: absoluteOf(AT) }]);
  expect(notesOf({ job_id: "j", state: "pending", record: {} })).toEqual([]);
});

it("lists each retro item against its Job by number, with the handle on hover", () => {
  const lessons: Lesson[] = [
    { id: "l-a", state: "open", job_id: "01K6Q3JOB3", handle: "3-retire-two-guides", at: AT, who: "fleet", statement: "A", evidence: [] },
    { id: "l-b", state: "open", job_id: "01K6Q3JOB3", handle: "3-retire-two-guides", at: AT, who: "drone", statement: "B", evidence: [] },
  ];
  expect(lessonRowsOf(lessons)).toEqual([
    {
      id: "l-a",
      jobId: "01K6Q3JOB3",
      who: "fleet",
      statement: "A",
      job: "Job 3",
      jobExact: "3-retire-two-guides",
      when: absoluteOf(AT),
      whenExact: AT,
    },
    {
      id: "l-b",
      jobId: "01K6Q3JOB3",
      who: "drone",
      statement: "B",
      job: "Job 3",
      jobExact: "3-retire-two-guides",
      when: absoluteOf(AT),
      whenExact: AT,
    },
  ]);
});

describe("where a retro item's fix lands", () => {
  const landed: JobRetro = {
    job_id: "j",
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
    const lessons: Lesson[] = [
      { id: "l", state: "open", job_id: "j", handle: "3-x", at: AT, who: "owner", statement: "A", evidence: [], lands_in: "kit" },
      { id: "l", state: "open", job_id: "j", handle: "3-x", at: AT, who: "owner", statement: "B", evidence: [] },
    ];
    expect(lessonRowsOf(lessons).map((one) => one.landsIn)).toEqual(["kit", undefined]);
  });

  it("narrows the retro lessons to one place, and keeps a row with none under All alone", () => {
    const rows = lessonRowsOf([
      { id: "l", state: "open", job_id: "j", handle: "3-x", at: AT, who: "owner", statement: "A", evidence: [], lands_in: "armada" },
      { id: "l", state: "open", job_id: "j", handle: "3-x", at: AT, who: "owner", statement: "B", evidence: [], lands_in: "kit" },
      { id: "l", state: "open", job_id: "j", handle: "3-x", at: AT, who: "owner", statement: "C", evidence: [], lands_in: "manifest" },
      { id: "l", state: "open", job_id: "j", handle: "3-x", at: AT, who: "owner", statement: "D", evidence: [] },
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
  const written: JobRetro = {
    job_id: "j",
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
      { id: "l-new", state: "open", job_id: "j", handle: "3-x", at: AT, who: "fleet", statement: "S", title: "T", what: "W", fix: "F", evidence: [] },
      { id: "l-old", state: "open", job_id: "j", handle: "3-x", at: AT, who: "owner", statement: "Old", evidence: [] },
    ]);
    expect(rows.map((one) => one.id)).toEqual(["l-new", "l-old"]);
    expect(rows[0]).toMatchObject({ title: "T", what: "W", fix: "F" });
    expect("title" in (rows[1] ?? {})).toBe(false);
  });
});

describe("what Agree and Disagree say they do", () => {
  it("names the place for each", () => {
    expect(agreeTipOf("armada")).toBe("Proposes a Job on Armada's repository");
    expect(agreeTipOf("manifest")).toBe("Proposes a Job on the Manifest's repository");
    expect(agreeTipOf("kit")).toBe("Saves it under Accepted");
    expect(DISAGREE_TIP).toBe("Discards it");
  });
});
