// Where Jobs' time went, for the Usage page and a Job's System tab: three reads, each answered by Fleet alone. A slice imports
// protocol, never another slice; `../api.ts` and `../bridge.ts` compose them.

import type { JobTime, Outcome, UsageHours, UsageTime } from "@armada/protocol";

/** What reading a window's time came to: Fleet's reading, or the refusal to word. */
export type UsageTimeRead =
  | {
      ok: true;
      value: UsageTime;
    }
  | {
      ok: false;
      outcome: Outcome;
    };

/** What reading the hours and builds came to. */
export type UsageHoursRead =
  | {
      ok: true;
      value: UsageHours;
    }
  | {
      ok: false;
      outcome: Outcome;
    };

/** What reading one Job's split came to. */
export type JobTimeRead =
  | {
      ok: true;
      value: JobTime;
    }
  | {
      ok: false;
      outcome: Outcome;
    };

export type UsageTimeApi = {
  /** `GET /usage/time`: the Jobs that ended in the last seven days, summed, with the slowest and the usual of each workflow. */
  getUsageTime: () => Promise<UsageTimeRead>;
  /** `GET /usage/hours`: the last 48 hours an hour at a time, and the builds that ran in them. */
  getUsageHours: () => Promise<UsageHoursRead>;
  /** `GET /sessions/time`: one Job's split against the usual of its workflow. */
  getJobTime: (sessionId: string) => Promise<JobTimeRead>;
};

export const USAGE_TIME_CHANNELS = {
  getUsageTime: "bridge:get-usage-time",
  getUsageHours: "bridge:get-usage-hours",
  getJobTime: "bridge:get-job-time",
} as const;
