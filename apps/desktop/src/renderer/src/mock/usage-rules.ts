// The Usage page's rules, answered as Fleet answers `GET /usage/rules`: the four rules and the trips each made in the last
// seven days. All of it is invented for the mock, the Jobs borrowed from the usage summary's. Mock only.

import type { UsageRuleTrip, UsageRules } from "@armada/protocol";

const HOUR_MS = 3_600_000;
const MERGER = "01JUSAGEMERGER0000000000";
const COCKPIT = "01JUSAGECOCKPIT000000000";
const DEBUG = "01JUSAGEDEBUG00000000000";
const MERGE_LINE = "01JUSAGEMERGELINE0000000";

const trip = (now: number, hoursAgo: number, session_id: string, figure: number, before: number, after?: number): UsageRuleTrip => ({
  session_id,
  at: new Date(now - hoursAgo * HOUR_MS).toISOString(),
  figure,
  burn_per_hour_before: before,
  ...(after === undefined ? {} : { burn_per_hour_after: after }),
});

export function usageRulesNow(now = Date.now()): UsageRules {
  return {
    since: new Date(now - 7 * 24 * HOUR_MS).toISOString(),
    until: new Date(now).toISOString(),
    rules: [
      {
        rule: "context",
        on: true,
        limit: { value: 300, unit: "thousand tokens" },
        trips: [
          trip(now, 2, MERGER, 351_000, 11.8),
          trip(now, 30, DEBUG, 531_000, 14.2, 13.9),
          trip(now, 52, COCKPIT, 338_000, 9.6, 6.1),
          trip(now, 101, MERGER, 318_000, 8.8, 9.7),
          trip(now, 140, MERGE_LINE, 305_000, 4.1, 2.6),
        ],
      },
      {
        rule: "subagents",
        on: true,
        limit: { value: 50, unit: "percent" },
        trips: [trip(now, 9, MERGER, 87, 12.4, 5.2), trip(now, 40, COCKPIT, 71, 8.1, 8.3), trip(now, 77, MERGE_LINE, 64, 5.6, 3.0), trip(now, 120, MERGER, 78, 10.2, 11.6)],
      },
      { rule: "cache_hit", on: false, limit: { value: 70, unit: "percent" }, trips: [] },
      {
        rule: "opus_reading",
        on: true,
        limit: { value: 20, unit: "turns" },
        trips: [trip(now, 21, DEBUG, 20, 15.6, 6.2), trip(now, 66, COCKPIT, 20, 7.4, 7.5), trip(now, 95, MERGER, 20, 9.9, 4.1)],
      },
    ],
  };
}
