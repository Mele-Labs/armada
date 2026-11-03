// The Usage page's Time view before Fleet has answered: the reading is held pending, so every panel draws its skeleton.

import type { Scenario } from "../moment";
import { usageWeek } from "./usage-week";

function build(): Scenario {
  return {
    ...usageWeek,
    name: "usage/time-loading",
    says: "The Usage page's Time view while the reading is still being made: skeletons in every panel",
    draft: {
      ...usageWeek.draft,
      time: "pending",
      builds: "pending",
    },
  };
}

export const usageTimeLoading: Scenario = build();
