import { describe, expect, it } from "vitest";

import { jobFactsOf, provideJobFacts } from "./job-facts";

describe("job facts", () => {
  it("answers a Job's facts as its pane last gave them", () => {
    let lines = ["State: working"];
    const forget = provideJobFacts("01JOB", () => lines);
    expect(jobFactsOf("01JOB")).toEqual(["State: working"]);
    lines = ["State: idle"];
    expect(jobFactsOf("01JOB")).toEqual(["State: idle"]);
    forget();
  });

  it("answers nothing for a Job no pane gave facts for, or one whose pane has gone", () => {
    expect(jobFactsOf("01NONE")).toBeUndefined();
    const forget = provideJobFacts("01GONE", () => ["Job: gone"]);
    forget();
    expect(jobFactsOf("01GONE")).toBeUndefined();
  });

  it("keeps the newer pane's facts where an older pane for the same Job goes after it arrives", () => {
    const forgetOld = provideJobFacts("01TWO", () => ["from: old"]);
    const forgetNew = provideJobFacts("01TWO", () => ["from: new"]);
    forgetOld();
    expect(jobFactsOf("01TWO")).toEqual(["from: new"]);
    forgetNew();
    expect(jobFactsOf("01TWO")).toBeUndefined();
  });
});
