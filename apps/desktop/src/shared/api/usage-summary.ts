// What a window of turns cost, for the Usage page: read once, as a window and the offset of the caller's local time ask
// it. A slice imports protocol, never another slice; `../api.ts` and `../bridge.ts` compose them.

import type { Outcome, UsageRules, UsageSummary } from "@armada/protocol";

/** A stretch of time as two instants, RFC3339. */
export type UsageWindow = {
  since: string;
  until: string;
};

/** What reading the summary came to: Fleet's summary, or the refusal to word. */
export type UsageSummaryRead =
  | {
      ok: true;
      value: UsageSummary;
    }
  | {
      ok: false;
      outcome: Outcome;
    };

/** What reading the usage rules came to: Fleet's rules with their trips, or the refusal to word. */
export type UsageRulesRead =
  | {
      ok: true;
      value: UsageRules;
    }
  | {
      ok: false;
      outcome: Outcome;
    };

export type UsageSummaryApi = {
  /**
   * `GET /usage/summary`: the last seven days, cut into days, weekdays and three-hour blocks `tzOffsetMinutes` ahead of UTC;
   * or, given a `window`, the turns in `[since, until)`, which is how a build's tokens are read.
   */
  getUsageSummary: (tzOffsetMinutes: number, window?: UsageWindow) => Promise<UsageSummaryRead>;
  /** `GET /usage/rules`: the four rules and the Jobs each told in the last seven days. */
  getUsageRules: () => Promise<UsageRulesRead>;
};

export const USAGE_SUMMARY_CHANNELS = {
  getUsageSummary: "bridge:get-usage-summary",
  getUsageRules: "bridge:get-usage-rules",
} as const;
