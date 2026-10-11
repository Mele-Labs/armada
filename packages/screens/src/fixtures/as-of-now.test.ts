import { expect, test } from "vitest";

import { asOfNow } from "./as-of-now";
import { NOW, STARTED_AT } from "./build/base";
import { completedSuccess } from "./build/terminal";
import { running } from "./build/running";

const HOUR = 3_600_000;

test("moves every instant by how far now is from the fixture's NOW", () => {
  const moved = asOfNow(running(), NOW + HOUR);
  expect(moved.job.started_at).toBe(new Date(Date.parse(STARTED_AT) + HOUR).toISOString());
  expect(moved.job.created_at).toBe(new Date(Date.parse(running().job.created_at) + HOUR).toISOString());
});

test("keeps a terminal Job's run time, since both ends move together", () => {
  const before = completedSuccess().job;
  const after = asOfNow(completedSuccess(), NOW + 500 * HOUR).job;
  expect(Date.parse(after.ended_at!) - Date.parse(after.started_at!)).toBe(
    Date.parse(before.ended_at!) - Date.parse(before.started_at!),
  );
});

test("leaves the builder's own fixture where it was", () => {
  const fixture = running();
  asOfNow(fixture, NOW + HOUR);
  expect(fixture.job.started_at).toBe(STARTED_AT);
});
