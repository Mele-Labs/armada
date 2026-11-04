// The Usage page with its Time view: the last 48 hours an hour a point on both views, over the same board as `usage/week`,
// and sixteen builds with a Builds panel and a marker each on the hour charts. Fleet answers the three time reads, and a
// build's tokens as a summary over its run, with the sketches in `../job-time.ts`, `../usage-builds.ts` and `../usage-hours.ts`.
// The walk `jobTime` plays it.

import type { Scenario } from "../moment";
import { jobTimeNow } from "../job-time";
import { usageBuildsNow } from "../usage-builds";
import { usageHoursNow } from "../usage-hours";
import { usageSummaryNow } from "../usage-summary";
import { usageOfBuild, wireHours, wireJobTime, wireTime } from "../usage-time-wire";
import { usageWeek } from "./usage-week";

function build(): Scenario {
  return {
    ...usageWeek,
    name: "usage/time",
    says: "The Usage page's Time view: the four figures hour by hour, the split of the window and the slowest Jobs, with the builds beside both views",
    behaves: (...args) => {
      // One clock for the window, so a build's start is the same instant in the hours and in the summary asked over its run.
      const now = Date.now();
      const time = jobTimeNow();
      const builds = usageBuildsNow(now);
      const hours = usageHoursNow(now);
      return {
        ...usageWeek.behaves?.(...args),
        getUsageTime: async () => ({
          ok: true,
          value: wireTime(time, now),
        }),
        getUsageHours: async () => ({
          ok: true,
          value: wireHours(hours, builds),
        }),
        getJobTime: async (sessionId) => ({
          ok: true,
          value: wireJobTime(time, sessionId, now),
        }),
        getUsageSummary: async (tzOffsetMinutes, window) => {
          const summary = usageSummaryNow(tzOffsetMinutes);
          const usage = window === undefined ? undefined : usageOfBuild(builds, window.since);
          return {
            ok: true,
            value:
              usage === undefined
                ? summary
                : {
                    ...summary,
                    total: usage,
                  },
          };
        },
      };
    },
  };
}

export const usageTime: Scenario = build();
