import { expect, test } from "vitest";
import type { ModSummary, TrackerTry } from "@armada/protocol";

import { trackerOf } from "./taught-source";

const row = (over: Partial<ModSummary> = {}): ModSummary => ({ name: "linear", kind: "tracker", enabled: true, valid: true, ...over });
const tried = (over: Partial<TrackerTry> = {}): TrackerTry => ({ at: "2026-10-14T09:00:00.000Z", manifest_id: "armada", passed: true, listed: 3, ...over });

test("a row with no try reads never, and stale only where Fleet marks it edited since", () => {
  expect(trackerOf(row()).tried).toEqual({ state: "never", words: "Not tried" });
  expect(trackerOf(row({ stale: true })).tried).toEqual({ state: "stale", words: "Stale" });
  expect(trackerOf(row({ stale: false })).tried.state).toBe("never");
});

test("a try Fleet kept reads passed with when and where, or failed with its reason", () => {
  expect(trackerOf(row({ tried: tried() })).tried).toEqual({ state: "passed", words: "Tried 14 Oct on armada" });
  expect(trackerOf(row({ tried: tried({ passed: false, reason: "`tracker-cli` is not installed" }) })).tried).toEqual({
    state: "failed",
    words: "Failed 14 Oct: `tracker-cli` is not installed",
  });
  expect(trackerOf(row({ tried: tried({ passed: false }) })).tried.words).toBe("Failed 14 Oct: no reason given");
});

test("a kept try wins over the stale mark, and validity is the row's own", () => {
  expect(trackerOf(row({ tried: tried(), stale: true })).tried.state).toBe("passed");
  expect(trackerOf(row({ valid: false, reason: "[search] has no argv" })).valid).toBe(false);
  expect(trackerOf(row()).valid).toBe(true);
});
