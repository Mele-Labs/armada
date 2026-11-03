// The builds of Bridge the Usage page reads its figures against: twelve over the last seven days, newest first. Mock only:
// no wire carries them yet, so these are sketches shaped to tell one story. Burn per turn, the model's wait, a tool's time and
// a pull request's wait hold steady; at the eighth build, a preview, a Check run takes about 40% longer and stays there. The
// newest build has taken too few turns to compare. Commits are invented.

import type { Build, TimeSplit, UsageBuilds } from "@armada/components";

import { jobTimeNow } from "./job-time";
import { JOBS, buildUsage } from "./usage-summary";

const HOUR = 3_600_000;
const DAY = 24 * HOUR;

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

/** Oldest first. */
const SEEDS: Seed[] = [
  build("4be7c19a03", 1980, 81200, 41.2, 5.2, 512, 1380),
  build("4be7c19a03", 2140, 79400, 40.6, 5.15, 505, 1410, {
    id: "d03a6e1",
    branches: [
      "armada/quick-dispatch",
    ],
  }),
  build("a90d3f6e52", 1760, 82800, 42.1, 5.3, 526, 1365),
  build("c6e18f2a07", 2210, 80100, 41.5, 5.25, 518, 1395),
  build("c6e18f2a07", 2050, 81500, 42.8, 5.2, 509, 1420, {
    id: "b81f04c",
    branches: [
      "armada/usage-time",
      "armada/job-retros",
    ],
  }),
  build("17c2b8d4f0", 1890, 82200, 41.9, 5.3, 521, 1385),
  build("e5604ab91c", 2320, 80600, 41.0, 5.25, 514, 1400),
  build("e5604ab91c", 2105, 81900, 42.3, 5.35, 731, 1390, {
    id: "7ac3d52",
    branches: [
      "armada/check-queue",
      "armada/slot-pool",
    ],
  }),
  build("3f9a1c20be", 1930, 80300, 41.4, 5.3, 724, 1415),
  build("58b2d0e7c1", 2260, 81100, 42.0, 5.25, 738, 1375),
  build("58b2d0e7c1", 1870, 82400, 41.6, 5.2, 729, 1405, {
    id: "e19b5f8",
    branches: [
      "armada/walk-takes",
    ],
  }),
  build("92d7e0a6b4", 18, 81000, 42.4, 5.3, 735, 1395),
];

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

const localDay = (at: number): string => {
  const day = new Date(at);
  const part = (n: number) => String(n).padStart(2, "0");
  return `${day.getFullYear()}-${part(day.getMonth() + 1)}-${part(day.getDate())}`;
};

export function usageBuildsNow(now = Date.now()): UsageBuilds {
  const tz = -new Date(now).getTimezoneOffset();
  const time = jobTimeNow();
  const six = JOBS.map((one) => one.id);
  const eight = [
    ...six,
    ...time.jobs.map((one) => one.id).filter((id) => !six.includes(id)),
  ];
  // The first began at seven in the morning, six days back; the last three hours ago.
  const morning = new Date(now);
  morning.setHours(7, 0, 0, 0);
  const first = morning.getTime() - 6 * DAY;
  const step = (now - 3 * HOUR - first) / (SEEDS.length - 1);
  const starts = SEEDS.map((_, at) => Math.round((first + at * step) / 60_000) * 60_000);
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
      week: time.weeks.reduce(
        (found, one, index) => (one.from <= localDay(starts[at]!) ? index : found),
        0,
      ),
      median: mean(splits),
    };
  });
  return builds.reverse();
}
