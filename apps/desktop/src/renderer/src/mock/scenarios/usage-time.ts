// The Usage page with its Time view: six weeks in which a Job's Checks and Pull request time grows and its Model time stays
// flat, over the same board as `usage/week`, and twelve builds with a Builds panel on both views. The walk `jobTime` plays it.

import type { Scenario } from "../moment";
import { jobTimeNow } from "../job-time";
import { usageBuildsNow } from "../usage-builds";
import { usageWeek } from "./usage-week";

function build(): Scenario {
  return {
    ...usageWeek,
    name: "usage/time",
    says: "The Usage page's Time view: the median Job week by week, the split of the window and the slowest Jobs, with the builds beside both views",
    draft: {
      ...usageWeek.draft,
      time: jobTimeNow(),
      builds: usageBuildsNow(),
    },
  };
}

export const usageTime: Scenario = build();
