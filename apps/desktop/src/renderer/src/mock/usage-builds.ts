// The builds of Bridge the Usage page reads its figures against: sixteen over the last 48 hours, newest first. Mock only:
// sketches, put on the wire by `usage-time-wire.ts`, shaped to tell one story. Burn per turn, the model's wait, a tool's time and
// a pull request's wait hold steady; at the tenth build, a preview, a Check run takes about 40% longer and stays there. The
// newest build has taken too few turns to compare. Commits are invented.

import type { Build, TimeSplit, UsageBuilds } from "@armada/components";

import { jobTimeNow } from "./job-time";
import { JOBS, buildUsage } from "./usage-summary";

const HOUR = 3_600_000;

type Seed = {
  /** The main commit the build stands on, ten characters. */
  commit: string;
  /** A preview build's own short commit, and the branches it holds. */
  preview?: {
    id: string;
    branches: string[];
  };
  turns: number;
  /** Units per turn, then seconds. */
  burn: number;
  model: number;
  tools: number;
  checks: number;
  pull_request: number;
};

const build = (
  commit: string,
  turns: number,
  burn: number,
  model: number,
  tools: number,
  checks: number,
  pull_request: number,
  preview?: Seed["preview"],
): Seed => ({
  commit,
  turns,
  burn,
  model,
  tools,
  checks,
  pull_request,
  ...(preview === undefined
    ? {}
    : {
        preview,
      }),
});

/** Oldest first. The eleventh is the preview where Checks steps up. */
const SEEDS: Seed[] = [
  build("4be7c19a03", 360, 81200, 41.2, 5.2, 512, 1380),
  build("4be7c19a03", 385, 79400, 40.6, 5.15, 505, 1410, {
    id: "d03a6e1",
    branches: [
      "armada/quick-dispatch",
    ],
  }),
  build("a90d3f6e52", 310, 82800, 42.1, 5.3, 526, 1365),
  build("c6e18f2a07", 395, 80100, 41.5, 5.25, 518, 1395),
  build("c6e18f2a07", 370, 81500, 42.8, 5.2, 509, 1420, {
    id: "b81f04c",
    branches: [
      "armada/usage-time",
      "armada/job-retros",
    ],
  }),
  build("17c2b8d4f0", 340, 82200, 41.9, 5.3, 521, 1385),
  build("8d41e3f7a6", 355, 80900, 41.3, 5.2, 516, 1405),
  build("6a0c93d1e8", 330, 81700, 42.2, 5.25, 507, 1390),
  build("e5604ab91c", 415, 80600, 41.0, 5.25, 514, 1400),
  build("e5604ab91c", 380, 81900, 42.3, 5.35, 731, 1390, {
    id: "7ac3d52",
    branches: [
      "armada/check-queue",
      "armada/slot-pool",
    ],
  }),
  build("3f9a1c20be", 345, 80300, 41.4, 5.3, 724, 1415),
  build("58b2d0e7c1", 405, 81100, 42.0, 5.25, 738, 1375),
  build("58b2d0e7c1", 335, 82400, 41.6, 5.2, 729, 1405, {
    id: "e19b5f8",
    branches: [
      "armada/walk-takes",
    ],
  }),
  build("b4f7a2e61d", 320, 80700, 41.8, 5.3, 733, 1398),
  build("71e0c5d98a", 350, 81600, 42.1, 5.25, 727, 1388),
  build("92d7e0a6b4", 18, 81000, 42.4, 5.3, 735, 1395),
];

/** Hours before now each build began, oldest first: eight to twelve a day, with the newest not yet an hour old. */
const AGO = [
  46.2, 43.1, 40.4, 37.5, 34.2, 31.6, 28.7, 25.9, 23.3, 20.6, 17.8, 14.4, 11.2, 8.0, 4.6, 0.9,
];

/** When each build began, oldest first, to the minute. */
export const buildStarts = (now: number): number[] =>
  AGO.map((ago) => Math.round((now - ago * HOUR) / 60_000) * 60_000);

const mean = (splits: readonly TimeSplit[]): TimeSplit => {
  const keys = Object.keys(splits[0]!) as (keyof TimeSplit)[];
  return Object.fromEntries(
    keys.map((key) => [
      key,
      splits.reduce((sum, one) => sum + one[key], 0) / splits.length,
    ]),
  ) as unknown as TimeSplit;
};
const sum = (splits: readonly TimeSplit[]): TimeSplit =>
  Object.fromEntries(
    Object.entries(mean(splits)).map(([key, value]) => [
      key,
      value * splits.length,
    ]),
  ) as unknown as TimeSplit;

export function usageBuildsNow(now = Date.now()): UsageBuilds {
  const tz = -new Date(now).getTimezoneOffset();
  const time = jobTimeNow();
  const six = JOBS.map((one) => one.id);
  const eight = [
    ...six,
    ...time.jobs.map((one) => one.id).filter((id) => !six.includes(id)),
  ];
  const starts = buildStarts(now);
  const builds: Build[] = SEEDS.map((seed, at) => {
    const newest = at === SEEDS.length - 1;
    const jobs = newest
      ? [
          six[5]!,
        ]
      : [
          ...new Set([
            six[at % 6]!,
            eight[(at * 3 + 1) % 8]!,
          ]),
        ];
    const splits = jobs.map((id) => time.jobs.find((one) => one.id === id)!.split);
    return {
      id: seed.preview?.id ?? seed.commit.slice(0, 7),
      commit: seed.commit,
      started: new Date(starts[at]!).toISOString(),
      branches: seed.preview?.branches ?? [],
      turns: seed.turns,
      burn: seed.burn,
      model: seed.model,
      tools: seed.tools,
      checks: seed.checks,
      pull_request: seed.pull_request,
      jobs,
      usage: buildUsage({
        units: (seed.turns * seed.burn) / 1_000_000,
        from: starts[at]!,
        to: newest ? now : starts[at + 1]!,
        seed: at + 2,
        tzOffsetMinutes: tz,
        now,
      }),
      split: sum(splits),
    };
  });
  return builds.reverse();
}
