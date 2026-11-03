// The Usage page with its Time view: six weeks in which a Job's Checks and Pull request time grows and its Model time stays
// flat, over the same board as `usage/week`. The walk `jobTime` plays it.

import type { Scenario } from "../moment";
import { jobTimeNow } from "../job-time";
import { usageWeek } from "./usage-week";

function build(): Scenario {
  return {
    ...usageWeek,
    name: "usage/time",
    says: "The Usage page's Time view: the median Job week by week, the split of the window and the slowest Jobs",
    draft: {
      ...usageWeek.draft,
      time: jobTimeNow(),
    },
  };
}

export const usageTime: Scenario = build();
