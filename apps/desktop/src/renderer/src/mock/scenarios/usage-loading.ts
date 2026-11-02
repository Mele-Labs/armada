// The Usage page before Fleet has answered: the summary read is held pending, so every panel draws its skeleton and
// nothing arrives. The same board as `usage/week`, which is the one with data to land on.

import type { Scenario } from "../moment";
import { usageWeek } from "./usage-week";

function build(): Scenario {
  return {
    ...usageWeek,
    name: "usage/loading",
    says: "The Usage page while Fleet's summary is still being read: skeletons in every panel",
    behaves: (...args) => ({
      ...usageWeek.behaves?.(...args),
      getUsageSummary: () => new Promise(() => {}),
      getUsageRules: () => new Promise(() => {}),
    }),
  };
}

export const usageLoading: Scenario = build();
