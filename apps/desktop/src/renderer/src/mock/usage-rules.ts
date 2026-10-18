// The Usage page's rules: Fleet's four usage rules and the trips each has made in the last seven days. All of it is
// invented for the mock, titles borrowed from the usage summary's Jobs. Mock only: Fleet serves none of this yet.

import type { UsageRule, UsageTrip } from "@armada/components";

const HOUR_MS = 3_600_000;
const MERGER = "01JUSAGEMERGER0000000000";
const COCKPIT = "01JUSAGECOCKPIT000000000";
const DEBUG = "01JUSAGEDEBUG00000000000";
const MERGE_LINE = "01JUSAGEMERGELINE0000000";

const trip = (now: number, hoursAgo: number, job_id: string, figure: string, figure_tip: string, before: number, after?: number): UsageTrip => ({
  job_id,
  at: new Date(now - hoursAgo * HOUR_MS).toISOString(),
  figure,
  figure_tip,
  before,
  after,
});

export function usageRulesNow(now = Date.now()): UsageRule[] {
  const context = "Context of the last turn";
  const share = "Share of the last hour's burn that ran in subagents";
  const reading = "Turns in a row on opus that only read and searched";
  return [
    {
      id: "context",
      name: "Context",
      limit: "300k, or 150k at 20 turns/h",
      limit_tip: "Last turn above 300k tokens, or above 150k while doing over 20 turns an hour",
      on: true,
      trips: [
        trip(now, 2, MERGER, "351k", context, 11.8),
        trip(now, 30, DEBUG, "531k", context, 14.2, 13.9),
        trip(now, 52, COCKPIT, "338k", context, 9.6, 6.1),
        trip(now, 101, MERGER, "318k", context, 8.8, 9.7),
        trip(now, 140, MERGE_LINE, "305k", context, 4.1, 2.6),
      ],
    },
    {
      id: "subagents",
      name: "Subagents",
      limit: "50% of the last hour",
      limit_tip: "Over 50% of the last hour's burn in subagents",
      on: true,
      trips: [
        trip(now, 9, MERGER, "87%", share, 12.4, 5.2),
        trip(now, 40, COCKPIT, "71%", share, 8.1, 8.3),
        trip(now, 77, MERGE_LINE, "64%", share, 5.6, 3.0),
        trip(now, 120, MERGER, "78%", share, 10.2, 11.6),
      ],
    },
    {
      id: "cache",
      name: "Cache",
      limit: "under 70% over 10 turns",
      limit_tip: "Cache hit rate under 70% over the last 10 turns",
      on: false,
      trips: [],
    },
    {
      id: "opus-reading",
      name: "Opus reading",
      limit: "20 turns only reading",
      limit_tip: "The last 20 turns on opus only read and searched",
      on: true,
      trips: [
        trip(now, 21, DEBUG, "20", reading, 15.6, 6.2),
        trip(now, 66, COCKPIT, "20", reading, 7.4, 7.5),
        trip(now, 95, MERGER, "20", reading, 9.9, 4.1),
      ],
    },
  ];
}
