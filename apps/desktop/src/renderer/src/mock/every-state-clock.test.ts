// The `every-state` rows' run times, on the real clock — #1505. The builders
// are dated a month back, so a running row read `194h 50m` until the mock moved
// each one onto the moment it was read.

import { expect, test } from "vitest";
import { elapsedOf } from "@armada/screens/src/Row";
import { NOW } from "@armada/screens/src/fixtures/build/base";

import { BUILDERS, scenarioNamed } from "./scenario";

type Builder = keyof typeof BUILDERS;

/** Each built row beside the builder it came from, read off the handle `asRow` gave it. */
const ROWS = scenarioNamed("every-state")!
  .state.jobs.filter((job) => job.id.includes("EVERYSTATE"))
  .map((job) => ({ job, built: BUILDERS[job.handle.replace(/^\d+-/, "") as Builder]().job }));

test("a running row reads its run time in minutes, as its builder does at its own now", () => {
  const running = ROWS.filter(({ job }) => job.ended_at === undefined && elapsedOf(job, Date.now()) !== undefined);
  expect(running.length).toBeGreaterThan(0);
  for (const { job, built } of running) {
    expect(elapsedOf(job, Date.now()), job.handle).toMatch(/^\d+m \d{2}s$/);
    // As far behind the real clock as the builder is behind `NOW`, give or take
    // the time between building the scenario and reading it here.
    const late = Date.now() - Date.parse(job.started_at!) - (NOW - Date.parse(built.started_at!));
    expect(late, job.handle).toBeGreaterThanOrEqual(0);
    expect(late, job.handle).toBeLessThan(60_000);
  }
});

test("a terminal row stops where its builder stops", () => {
  const over = ROWS.filter(({ job }) => job.ended_at !== undefined);
  expect(over.length).toBeGreaterThan(0);
  for (const { job, built } of over) {
    expect(elapsedOf(job, Date.now()), job.handle).toBe(elapsedOf(built, NOW));
    expect(job.ended_at, job.handle).not.toBe(built.ended_at);
  }
});
