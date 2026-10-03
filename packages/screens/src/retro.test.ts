// What a retro and the Lessons listing are drawn from — `retro.ts`.
import { describe, expect, it } from "vitest";

import type { JobRetro, Lesson } from "@armada/protocol";

import { absoluteOf } from "./duration";
import { itemsOf, lessonRowsOf, notesOf, statusOf } from "./retro";

const AT = "2026-10-02T21:12:00.000Z";

const RETRO: JobRetro = {
  job_id: "01K6Q3JOB3",
  state: "written",
  items: [
    { who: "fleet", statement: "The gate failed out_of_bounds.", evidence: ["check:1", "check:9"] },
    { who: "owner", statement: "Helm acted with curl.", evidence: ["act:2", "waited:1", "asked:1"] },
    { who: "drone", statement: "It never saw its test pass.", evidence: ["said:1", "refusal:1", "not_met:1"] },
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
    { job_id: "01K6Q3JOB3", handle: "3-retire-two-guides", at: AT, who: "fleet", statement: "A", evidence: [] },
    { job_id: "01K6Q3JOB3", handle: "3-retire-two-guides", at: AT, who: "drone", statement: "B", evidence: [] },
  ];
  expect(lessonRowsOf(lessons)).toEqual([
    {
      id: "01K6Q3JOB3:0",
      jobId: "01K6Q3JOB3",
      who: "fleet",
      statement: "A",
      job: "Job 3",
      jobExact: "3-retire-two-guides",
      when: absoluteOf(AT),
      whenExact: AT,
    },
    {
      id: "01K6Q3JOB3:1",
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
