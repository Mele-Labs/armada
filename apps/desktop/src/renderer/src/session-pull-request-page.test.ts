// The pure parts of the pull request panel's comment box: which writes may be sent with no words,
// and how a check's seconds and an instant's age are worded.

import { expect, test } from "vitest";
import { agoOf, mayWrite, ranFor } from "@armada/components";

test("a comment and a request for changes need words, and an approval does not", () => {
  expect(mayWrite("comment", "")).toBe(false);
  expect(mayWrite("comment", " \n ")).toBe(false);
  expect(mayWrite("request_changes", "")).toBe(false);
  expect(mayWrite("comment", "Looks right.")).toBe(true);
  expect(mayWrite("request_changes", "Needs a default.")).toBe(true);
  expect(mayWrite("approve", "")).toBe(true);
});

test("a check's seconds read as the Job board reads a span", () => {
  expect(ranFor(45)).toBe("45s");
  expect(ranFor(131)).toBe("2m 11s");
  expect(ranFor(242)).toBe("4m 02s");
  expect(ranFor(3780)).toBe("1h 03m");
});

test("how long ago is the nearest unit, and absent for an instant that will not parse", () => {
  const now = Date.parse("2026-10-13T12:00:00Z");
  expect(agoOf("2026-10-13T11:59:30Z", now)).toBe("<1m ago");
  expect(agoOf("2026-10-13T11:48:00Z", now)).toBe("12m ago");
  expect(agoOf("2026-10-13T09:55:00Z", now)).toBe("2h ago");
  expect(agoOf("2026-10-10T12:00:00Z", now)).toBe("3d ago");
  expect(agoOf("not a time", now)).toBeUndefined();
  expect(agoOf(undefined, now)).toBeUndefined();
});
