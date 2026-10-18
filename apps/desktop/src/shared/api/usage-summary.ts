// What a window of turns cost, for the Usage page: read once, as a window and the offset of the caller's local time ask
// it. A slice imports protocol, never another slice; `../api.ts` and `../bridge.ts` compose them.

import type { Outcome, UsageSummary } from "@armada/protocol";

/** What reading the summary came to: Fleet's summary, or the refusal to word. */
export type UsageSummaryRead = { ok: true; value: UsageSummary } | { ok: false; outcome: Outcome };

export type UsageSummaryApi = {
  /** `GET /usage/summary`: the last seven days, cut into days, weekdays and three-hour blocks `tzOffsetMinutes` ahead of UTC. */
  getUsageSummary: (tzOffsetMinutes: number) => Promise<UsageSummaryRead>;
};

export const USAGE_SUMMARY_CHANNELS = {
  getUsageSummary: "bridge:get-usage-summary",
} as const;
