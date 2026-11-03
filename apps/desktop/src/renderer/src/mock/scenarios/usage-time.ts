// The Usage page with its Time view: the last 48 hours an hour a point on both views, over the same board as `usage/week`,
// and sixteen builds with a Builds panel and a marker each on the hour charts. The walk `jobTime` plays it.

import type { Scenario } from "../moment";
import { jobTimeNow } from "../job-time";
import { usageBuildsNow } from "../usage-builds";
import { usageHoursNow } from "../usage-hours";
import { usageWeek } from "./usage-week";

function build(): Scenario {
  return {
    ...usageWeek,
    name: "usage/time",
    says: "The Usage page's Time view: the four figures hour by hour, the split of the window and the slowest Jobs, with the builds beside both views",
    draft: {
      ...usageWeek.draft,
      time: jobTimeNow(),
      builds: usageBuildsNow(),
      hours: usageHoursNow(),
    },
  };
}

export const usageTime: Scenario = build();
