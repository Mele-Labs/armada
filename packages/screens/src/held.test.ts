// How long a checkout has been held, as Cleanup's tiles say it.

import { expect, test } from "vitest";

import { sitting } from "./held";

const NOW = Date.parse("2026-09-03T12:00:00Z");

test("an age is coarse, and reads in the unit a person decides in", () => {
  expect(sitting("2026-09-03T11:59:40Z", NOW)).toBe("under a minute");
  expect(sitting("2026-09-03T11:59:00Z", NOW)).toBe("1 minute");
  expect(sitting("2026-09-03T11:38:00Z", NOW)).toBe("22 minutes");
  expect(sitting("2026-09-03T11:00:00Z", NOW)).toBe("1 hour");
  expect(sitting("2026-09-03T04:00:00Z", NOW)).toBe("8 hours");
  expect(sitting("2026-09-02T12:00:00Z", NOW)).toBe("1 day");
  expect(sitting("2026-08-30T09:14:00Z", NOW)).toBe("4 days");
});

/**
 * It rounds down and never up. `last_moved_at` is already a floor — armada
 * moved the job then, and the files were written at or before it — so rounding
 * up would turn a floor into a claim.
 */
test("an age rounds down, because the stamp it is measured from is a floor", () => {
  // Twenty-three and a half hours is not "1 day".
  expect(sitting("2026-09-02T12:30:00Z", NOW)).toBe("23 hours");
  // And three days and twenty-two hours is not "4 days".
  expect(sitting("2026-08-30T14:00:00Z", NOW)).toBe("3 days");
});

/**
 * A stamp that will not parse says nothing rather than showing an age measured
 * from zero — the convention `instant` sets. A stamp in the future is the same
 * refusal: a clock disagreeing is not an age.
 */
test("an unreadable or future stamp draws no age at all", () => {
  expect(sitting("not a date", NOW)).toBeNull();
  expect(sitting("2026-09-04T12:00:00Z", NOW)).toBeNull();
});
