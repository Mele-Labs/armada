// The Usage page's Time view before Fleet has answered: the three time reads are held unanswered, so every panel draws its skeleton.

import type { Scenario } from "../moment";
import { usageWeek } from "./usage-week";

function build(): Scenario {
  return {
    ...usageWeek,
    name: "usage/time-loading",
    says: "The Usage page's Time view while the reading is still being made: skeletons in every panel",
    behaves: (...args) => ({
      ...usageWeek.behaves?.(...args),
      getUsageTime: () => new Promise(() => {}),
      getUsageHours: () => new Promise(() => {}),
      getJobTime: () => new Promise(() => {}),
    }),
  };
}

export const usageTimeLoading: Scenario = build();
