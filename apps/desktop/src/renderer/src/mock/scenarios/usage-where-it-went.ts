// The Usage page under the new weights, by model and by cache lifetime, with a Spent on view: what the tokens went on over a
// chosen window, and who spent them, for all Jobs or one. Over the same board as `usage/week`; Fleet's answers are in
// `../usage-spent.ts`, and the page joins the Jobs to their titles as it does Fleet's. The walk `usage-where-it-went` plays it.

import type { Scenario } from "../moment";
import { reweighedSummaryNow, wireSpentNow } from "../usage-spent";
import { usageWeek } from "./usage-week";

function build(): Scenario {
  return {
    ...usageWeek,
    name: "usage/where-it-went",
    says: "The Usage page weighed by model and cache lifetime, with a Spent on view of what the tokens went on and who spent them",
    behaves: (...args) => ({
      ...usageWeek.behaves?.(...args),
      getUsageSummary: async (tzOffsetMinutes) => ({
        ok: true,
        value: reweighedSummaryNow(tzOffsetMinutes),
      }),
      getUsageSpent: async () => ({
        ok: true,
        value: wireSpentNow(),
      }),
    }),
  };
}

export const usageWhereItWent: Scenario = build();
