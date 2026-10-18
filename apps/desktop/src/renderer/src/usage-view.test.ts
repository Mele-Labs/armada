// What the Usage page works out from Fleet's summary: the labels, the shares, the seven local days, the weekday grid and the
// two kinds of delta, each of which is absent where there is nothing real to move from.

import type { UsageSummary } from "@armada/protocol";
import {
  BOUNDS,
  M,
  bashLabel,
  chartDays,
  dayLabel,
  familyOf,
  foldModels,
  heatOf,
  localDay,
  peakCell,
  pct,
  percentDelta,
  pointsDelta,
  rowsOf,
  shareOf,
  shareRows,
  stepOf,
  tickStep,
  toolLabel,
  tokenRows,
  weekdayOf,
} from "@armada/components/src/compositions/UsagePage/usage-view";
import { expect, test } from "vitest";

import { usageSummaryNow } from "./mock/usage-summary";

const NOW = Date.parse("2026-10-17T15:00:00Z");

test("a share under a tenth of a percent says so, and nothing divides by nothing", () => {
  expect(pct(0.04)).toBe("<0.1%");
  expect(pct(0)).toBe("0.0%");
  expect(pct(79)).toBe("79.0%");
  expect(shareOf(5, 0)).toBe(0);
  expect(M(305)).toBe("305.0");
});

test("the tool, Bash use and token labels are the approved copy", () => {
  expect(toolLabel("none")).toBe("no tool");
  expect(toolLabel("Bash")).toBe("Bash");
  expect(toolLabel("mcp__armada-fleet__waiting_for")).toBe("fleet waiting_for");
  expect(toolLabel("mcp__gitnexus__query")).toBe("query");
  expect(bashLabel("forge")).toBe("GitHub");
  expect(bashLabel("read_search")).toBe("Read and search files");
  expect(bashLabel("loops_waits")).toBe("Loops, waits and Fleet calls");
  const tokens = tokenRows({ by_token: { input: 1, cache_creation: 30, cache_read: 100, output: 20 } } as never).map((one) => one.name);
  expect(tokens).toEqual(["cache read", "cache creation", "output", "input"]);
});

test("model ids fold into their family, in the order the chart stacks them", () => {
  expect(familyOf("claude-sonnet-4-5")).toBe("sonnet");
  expect(familyOf("something-else")).toBe("something-else");
  const folded = foldModels([
    { name: "claude-opus-4", units: 10 },
    { name: "claude-sonnet-4", units: 5 },
    { name: "claude-sonnet-4-5", units: 7 },
    { name: "mystery", units: 99 },
  ]);
  expect(folded).toEqual([
    { name: "sonnet", units: 12 },
    { name: "opus", units: 10 },
    { name: "mystery", units: 99 },
  ]);
});

test("rows with nothing in them are left out of a share cut", () => {
  expect(shareRows([{ name: "a", units: 50 }, { name: "b", units: 0 }], 100)).toEqual([{ name: "a", share: 50 }]);
});

test("the chart is the seven local days ending on the one before `until`, with the days Fleet left out as none", () => {
  const summary = { until: "2026-10-18T00:00:00Z", tz_offset_minutes: 0 };
  const days = chartDays(summary, [{ day: "2026-10-12", units: 3 }, { day: "2026-10-17", units: 9, by_model: [{ name: "claude-opus-4", units: 9 }] }]);
  expect(days.map((one) => one.day)).toEqual(["2026-10-11", "2026-10-12", "2026-10-13", "2026-10-14", "2026-10-15", "2026-10-16", "2026-10-17"]);
  expect(days.map((one) => one.units)).toEqual([0, 3, 0, 0, 0, 0, 9]);
  expect(days[6]!.byModel).toEqual([{ name: "opus", units: 9 }]);
  expect(days.map((one) => one.weekday)).toEqual([6, 0, 1, 2, 3, 4, 5]);
});

test("a local day is cut by the caller's offset, not UTC's", () => {
  expect(localDay("2026-10-17T23:30:00Z", 120)).toBe("2026-10-18");
  expect(localDay("2026-10-17T23:30:00Z", -300)).toBe("2026-10-17");
  expect(weekdayOf("2026-10-12")).toBe(0);
  expect(weekdayOf("2026-10-18")).toBe(6);
  expect(dayLabel("2026-10-17")).toBe("Sat Oct 17");
});

test("the hour grid is turned from a row a weekday into a row a block, Monday first", () => {
  const grid = [0, 1, 2, 3, 4, 5, 6].map((weekday) => Array.from({ length: 8 }, (_, block) => weekday * 10 + block));
  const heat = heatOf(grid);
  expect(heat).toHaveLength(8);
  expect(heat[0]).toEqual([0, 10, 20, 30, 40, 50, 60]);
  expect(heat[7]![6]).toBe(67);
  expect(peakCell(heat)).toEqual({ d: 6, b: 7 });
  expect(heatOf([])[3]).toEqual([0, 0, 0, 0, 0, 0, 0]);
});

test("the map's steps run on the four bounds, and none is nothing", () => {
  expect(BOUNDS).toEqual([25, 50, 75]);
  expect([0, 0.2, 25, 25.1, 50, 75, 75.1].map(stepOf)).toEqual([0, 1, 1, 2, 2, 3, 4]);
});

test("the axis ticks stay within four steps", () => {
  expect(tickStep(305)).toBe(100);
  expect(tickStep(60)).toBe(20);
  expect(tickStep(0)).toBeGreaterThan(0);
});

test("a delta is absent where there was nothing before, or it did not move", () => {
  expect(pointsDelta(70, 66, true)).toEqual({ up: true, points: 4 });
  expect(pointsDelta(56, 61, true)).toEqual({ up: false, points: 5 });
  expect(pointsDelta(70, 66, false)).toBeUndefined();
  expect(pointsDelta(66.2, 66, true)).toBeUndefined();
  expect(percentDelta(292.3, 305)?.up).toBe(false);
  expect(percentDelta(5, 0)).toBeUndefined();
  expect(percentDelta(5, 5)).toBeUndefined();
});

test("the list is Fleet's Jobs under the titles held, and a Job with none shows the tail of its id", () => {
  const summary: UsageSummary = usageSummaryNow(0, NOW);
  const titles = new Map([[summary.sessions[0]!.session_id, "Merger"]]);
  const rows = rowsOf(summary, titles);
  expect(rows).toHaveLength(summary.sessions.length);
  expect(rows[0]).toMatchObject({ title: "Merger", known: true });
  expect(rows[1]).toMatchObject({ title: summary.sessions[1]!.session_id.slice(-6), known: false });
});

test("the mock's summary is Fleet's shape over the measured baseline, and its parts add up", () => {
  const summary = usageSummaryNow(0, NOW);
  expect(summary.total.units).toBeCloseTo(1662.2, 1);
  expect(summary.total.hour_grid).toHaveLength(7);
  expect(summary.total.hour_grid[0]).toHaveLength(8);
  const tools = summary.total.by_first_tool.reduce((a, b) => a + b.units, 0);
  const bash = summary.total.by_bash_use.reduce((a, b) => a + b.units, 0);
  expect(tools).toBeCloseTo(summary.total.units, 1);
  expect(bash).toBeCloseTo(summary.total.by_first_tool.find((one) => one.name === "Bash")!.units, 1);
  expect(summary.total.by_day).toHaveLength(7);
  for (const job of summary.sessions) expect(job.usage.by_first_tool.reduce((a, b) => a + b.units, 0)).toBeCloseTo(job.usage.units, 1);
});
