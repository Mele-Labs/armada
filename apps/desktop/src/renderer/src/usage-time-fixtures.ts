// A small Time view for the Usage page's tests: three Jobs, three builds and 48 hours, each shaped for one thing a test reads.
// The mock's own data moves with the clock and the zone, so a figure a test names would not stay put there.

import type { Build, TimeJob, TimeSplit, UsageHours, UsageTime } from "@armada/components";
import type { UsageHour } from "@armada/components";

import { usageBuildsNow } from "./mock/usage-builds";

const MINUTE = 60;
const HOUR_MS = 3_600_000;

/** A split from minutes by bucket, every other bucket none. */
export const minutes = (spent: Partial<Record<keyof TimeSplit, number>>): TimeSplit => ({
  model: 0,
  tools: 0,
  checks: 0,
  subagents: 0,
  slot: 0,
  you: 0,
  pull_request: 0,
  stopped: 0,
  idle: 0,
  ...Object.fromEntries(Object.entries(spent).map(([bucket, m]) => [bucket, m * MINUTE])),
});

const FEATURE = "Build a feature";

/** Against the usual of Build a feature: Model just under twice it, Checks and Pull request far over it, Slot three times it but under ten minutes past. */
export const SLOW: TimeJob = {
  id: "job-slow",
  title: "Merge a pull request",
  workflow: FEATURE,
  split: minutes({
    model: 79,
    tools: 22,
    checks: 148,
    subagents: 12,
    slot: 13,
    you: 25,
    pull_request: 121,
  }),
};
export const FIX: TimeJob = {
  id: "job-fix",
  title: "Fix the stale label",
  workflow: "Fix a bug",
  split: minutes({
    model: 22,
    tools: 9,
    checks: 8,
    you: 14,
    pull_request: 10,
    stopped: 4,
  }),
};
/** A workflow the window has no usual for. */
export const ODD: TimeJob = {
  id: "job-odd",
  title: "Rename the helper",
  workflow: "Clean up",
  split: minutes({
    model: 17,
    tools: 11,
    checks: 33,
  }),
};

export const TIME: UsageTime = {
  total: minutes({
    model: 500,
    tools: 200,
    checks: 600,
    you: 300,
  }),
  jobs: [
    SLOW,
    FIX,
    ODD,
  ],
  usual: {
    [FEATURE]: minutes({
      model: 40,
      tools: 14,
      checks: 12,
      subagents: 12,
      slot: 4,
      you: 25,
      pull_request: 12,
      stopped: 5,
      idle: 10,
    }),
    "Fix a bug": minutes({
      model: 22,
      tools: 9,
      checks: 8,
      you: 14,
      pull_request: 10,
    }),
  },
};

/** The page's first hour: local midnight of Nov 2, so the hours read the same in any zone. */
export const FIRST = new Date(2026, 10, 2, 0, 0);

/** Where an hour offset from the first hour falls, as an instant. */
export const hoursIn = (offset: number): string => new Date(FIRST.getTime() + offset * HOUR_MS).toISOString();

/** Burn through hours 8 to 20 and 32 to 44, nothing overnight; a Check run's time steps up from hour 33; hour 10 has no pull request. */
export const HOURS: UsageHours = Array.from(
  {
    length: 48,
  },
  (_, at): UsageHour => {
    const working = (at >= 8 && at <= 20) || (at >= 32 && at <= 44);
    return working
      ? {
          at: hoursIn(at),
          burn: [
            {
              name: "sonnet",
              units: 5,
            },
          ],
          model: 40,
          tools: 5,
          checks: at >= 33 ? 730 : 510,
          ...(at === 10
            ? {}
            : {
                pull_request: 1400,
              }),
        }
      : {
          at: hoursIn(at),
        };
  },
);

const mockBuild: Build = usageBuildsNow(Date.parse("2026-11-03T15:00:00Z"))[0]!;
const build = (
  over: Pick<Build, "id" | "commit" | "started" | "branches" | "turns" | "checks" | "jobs" | "split">,
): Build => ({
  ...mockBuild,
  burn: 80000,
  model: 40,
  tools: 5,
  pull_request: 1400,
  ...over,
});

/** Newest first: a main build with too few turns, a preview where Checks stepped up 43%, and the main build it was cut from. */
export const BUILDS: readonly Build[] = [
  build({
    id: "c3d4e5f",
    commit: "c3d4e5f6a7",
    started: new Date(2026, 10, 3, 14, 36).toISOString(),
    branches: [],
    turns: 20,
    checks: 731,
    jobs: [
      ODD.id,
    ],
    split: minutes({
      model: 5,
    }),
  }),
  build({
    id: "7ac3d52",
    commit: "e5604ab91c",
    started: new Date(2026, 10, 3, 9, 5).toISOString(),
    branches: [
      "armada/check-queue",
      "armada/slot-pool",
    ],
    turns: 380,
    checks: 731,
    jobs: [
      SLOW.id,
      FIX.id,
    ],
    split: minutes({
      model: 30,
      checks: 90,
    }),
  }),
  build({
    id: "e5604ab",
    commit: "e5604ab91c",
    started: new Date(2026, 10, 2, 18, 0).toISOString(),
    branches: [],
    turns: 415,
    checks: 512,
    jobs: [
      SLOW.id,
    ],
    split: minutes({
      model: 10,
    }),
  }),
];
