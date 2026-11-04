// The usage summary is one request, and what crosses from the renderer is only an offset in minutes.

import { expect, test } from "vitest";

import { usageSummaryPath } from "./usage-summary";
import { offsetOf, windowOf } from "./usage-summary-channels";

test("the path carries the caller's offset and nothing else", () => {
  expect(usageSummaryPath(120)).toBe("/usage/summary?tz_offset_minutes=120");
  expect(usageSummaryPath(-300)).toBe("/usage/summary?tz_offset_minutes=-300");
});

test("only a whole offset within a day either way is passed on", () => {
  expect(offsetOf(-300)).toBe(-300);
  expect(offsetOf(0)).toBe(0);
  for (const bad of [
    1441,
    -1441,
    1.5,
    "120",
    null,
    undefined,
    NaN,
  ])
    expect(offsetOf(bad)).toBeNull();
});

test("a window adds its two instants, encoded, to the offset", () => {
  expect(
    usageSummaryPath(60, {
      since: "2026-11-03T09:00:00Z",
      until: "2026-11-03T10:30:00+01:00",
    }),
  ).toBe(
    "/usage/summary?tz_offset_minutes=60&since=2026-11-03T09%3A00%3A00Z&until=2026-11-03T10%3A30%3A00%2B01%3A00",
  );
});

test("a window is passed on only as two instants in order, and none at all is no window", () => {
  const ok = {
    since: "2026-11-03T09:00:00Z",
    until: "2026-11-03T10:00:00Z",
  };
  expect(windowOf(ok)).toEqual(ok);
  expect(windowOf(undefined)).toBeUndefined();
  for (const bad of [
    null,
    "2026-11-03T09:00:00Z",
    {},
    {
      since: ok.since,
    },
    {
      since: 1,
      until: 2,
    },
    {
      since: "lately",
      until: ok.until,
    },
    {
      since: ok.until,
      until: ok.since,
    },
    {
      since: ok.since,
      until: ok.since,
    },
  ])
    expect(windowOf(bad)).toBeNull();
});
