// The Usage page under the new weights, and what its tokens were spent on, as Fleet answers them on the wire.
//
// **Reweighed from the week's summary**, which was summed with every model weighed alike and every cache write at 1.25. A
// model now weighs Haiku 1, Sonnet 3, Opus and Fable 5, and an hour's cache write 2. The summary holds units by model and the
// cache writes' units, but not which writes were the hour's, so the mock takes seven in ten of them to be, and scales each
// breakdown by its own model mix. Long contexts ran on the heavier models, so the share above 200k is taken up by a twelfth,
// a sketch. **Every subagent share is the one Spent by draws**, weighed spend under one definition: the week's tile and each
// Job's column read the 7 day split below, and the week before reads 23%, so the 14 days come to the measured 25%.
//
// **What it was spent on is measured** over 14 days of this Mac's transcripts, the whole and who spent it. The 7 day and
// 24 hour splits, and each Job's own, are sketches around it.

import type { SpentSplit, SpentWindow, UsageSpent } from "@armada/components";
import type { NamedUnits, UsageBreakdown, UsageSpentUnits, UsageSummary } from "@armada/protocol";

import { JOBS, usageSummaryNow } from "./usage-summary";

const MODEL_WEIGHT: Readonly<Record<string, number>> = {
  haiku: 1,
  sonnet: 3,
  opus: 5,
  fable: 5,
};
/** Of the cache writes' units, the share that were an hour's. */
const HOUR_WRITES = 0.7;
/** An hour's write at 2 where it was summed at 1.25. */
const HOUR_LIFT = 2 / 1.25;
/** The week before's subagent share, under the same definition. */
const PRIOR_SUBAGENTS = 0.23;
const ABOVE_LIFT = 13 / 12;

const weightOf = (model: string): number =>
  Object.entries(MODEL_WEIGHT).find(([family]) => model.toLowerCase().includes(family))?.[1] ?? 3;

/** The model mix's weight: units under the new weights per unit under the old. */
function mixOf(byModel: readonly NamedUnits[]): number {
  const all = byModel.reduce((sum, one) => sum + one.units, 0);
  return all > 0 ? byModel.reduce((sum, one) => sum + one.units * weightOf(one.name), 0) / all : 1;
}

/** One breakdown under the new weights. */
function reweighed(usage: UsageBreakdown, subagents: number): UsageBreakdown {
  const mix = mixOf(usage.by_model);
  const writes = usage.by_token.cache_creation;
  const lift = usage.units > 0 ? 1 + ((HOUR_LIFT - 1) * HOUR_WRITES * writes) / usage.units : 1;
  const by = mix * lift;
  const scaled = (rows: readonly NamedUnits[]): NamedUnits[] =>
    rows.map((one) => ({
      name: one.name,
      units: one.units * by,
    }));
  const models = (rows: readonly NamedUnits[]): NamedUnits[] =>
    rows
      .map((one) => ({
        name: one.name,
        units: one.units * weightOf(one.name) * lift,
      }))
      .sort((a, b) => b.units - a.units);
  const units = usage.units * by;
  return {
    units,
    subagent_units: subagents * units,
    above_200k_units: (usage.above_200k_units / usage.units) * ABOVE_LIFT * units,
    above_300k_units: (usage.above_300k_units / usage.units) * ABOVE_LIFT * units,
    // The hour's writes beside the five minute ones, under a key the wire does not carry yet.
    by_token: Object.fromEntries([
      [
        "input",
        usage.by_token.input * mix,
      ],
      [
        "cache_read",
        usage.by_token.cache_read * mix,
      ],
      [
        "output",
        usage.by_token.output * mix,
      ],
      [
        "cache_creation",
        writes * (1 - HOUR_WRITES) * mix,
      ],
      [
        "cache_creation_1h",
        writes * HOUR_WRITES * HOUR_LIFT * mix,
      ],
    ]) as UsageBreakdown["by_token"],
    by_model: models(usage.by_model),
    by_first_tool: scaled(usage.by_first_tool),
    by_bash_use: scaled(usage.by_bash_use),
    by_day: usage.by_day.map((one) => ({
      day: one.day,
      units: one.units * by,
      ...(one.by_model === undefined
        ? {}
        : {
            by_model: models(one.by_model),
          }),
    })),
    hour_grid: usage.hour_grid.map((row) => row.map((cell) => cell * by)),
  };
}

/** The week's summary as Fleet would answer it under the new weights. */
export function reweighedSummaryNow(tzOffsetMinutes: number, now = Date.now()): UsageSummary {
  const old = usageSummaryNow(tzOffsetMinutes, now);
  const total = reweighed(old.total, subagentsOf(WEEK));
  const by = total.units / old.total.units;
  return {
    ...old,
    total,
    sessions: old.sessions
      .map((one) => ({
        ...one,
        subagent_share: subagentsOf(JOB_SHARES[one.session_id]!),
        usage: reweighed(one.usage, subagentsOf(JOB_SHARES[one.session_id]!)),
      }))
      .sort((a, b) => b.usage.units - a.usage.units),
    prior: {
      ...old.prior,
      units: old.prior.units * by,
      subagent_share: PRIOR_SUBAGENTS,
      above_200k_share: old.prior.above_200k_share * ABOVE_LIFT,
      last_day: {
        ...old.prior.last_day,
        units: old.prior.last_day.units * by,
      },
      day_before: {
        ...old.prior.day_before,
        units: old.prior.day_before.units * by,
      },
    },
  };
}

/** Shares in `SPENT_ON` order, then `SPENT_BY` order, each out of 100. */
type Shares = {
  on: readonly number[];
  by: readonly number[];
};

/** The subagents' share of a split, 0 to 1. */
const subagentsOf = (shares: Shares): number =>
  shares.by[1]! / shares.by.reduce((a, b) => a + b, 0);

function splitOf(units: number, shares: Shares): SpentSplit {
  const part = (list: readonly number[], at: number) =>
    (units * list[at]!) / list.reduce((a, b) => a + b, 0);
  return {
    on: {
      start_up: part(shares.on, 0),
      reads: part(shares.on, 1),
      own_words: part(shares.on, 2),
      output: part(shares.on, 3),
      cold_cache: part(shares.on, 4),
      announced: part(shares.on, 5),
      other_results: part(shares.on, 6),
      messages: part(shares.on, 7),
      background: part(shares.on, 8),
    },
    by: {
      main: part(shares.by, 0),
      subagents: part(shares.by, 1),
      background: part(shares.by, 2),
    },
  };
}

/** Measured, 14 days. */
const FORTNIGHT: Shares = {
  on: [
    22,
    21,
    19,
    10,
    9,
    8,
    6,
    3,
    2,
  ],
  by: [
    73,
    25,
    2,
  ],
};
const WEEK: Shares = {
  on: [
    21,
    22,
    19,
    10,
    10,
    8,
    5,
    3,
    2,
  ],
  by: [
    71,
    27,
    2,
  ],
};
const DAY: Shares = {
  on: [
    18,
    24,
    20,
    11,
    12,
    7,
    5,
    2,
    1,
  ],
  by: [
    68,
    31,
    1,
  ],
};

/** Each Job's own split, by id: Debug Job 14 ran long and idle, so a cold cache rewrote its context most. */
const JOB_SHARES: Readonly<Record<string, Shares>> = {
  "01JUSAGEMERGER0000000000": {
    on: [
      17,
      26,
      18,
      9,
      6,
      9,
      9,
      4,
      2,
    ],
    by: [
      58,
      40,
      2,
    ],
  },
  "01JUSAGECOCKPIT000000000": {
    on: [
      20,
      23,
      20,
      10,
      7,
      8,
      7,
      3,
      2,
    ],
    by: [
      63,
      35,
      2,
    ],
  },
  "01JUSAGEDEBUG00000000000": {
    on: [
      14,
      18,
      17,
      8,
      28,
      6,
      6,
      2,
      1,
    ],
    by: [
      79,
      20,
      1,
    ],
  },
  "01JUSAGEMERGELINE0000000": {
    on: [
      30,
      15,
      17,
      9,
      8,
      12,
      5,
      2,
      2,
    ],
    by: [
      68,
      30,
      2,
    ],
  },
  "01JUSAGESYNCING000000000": {
    on: [
      24,
      22,
      18,
      10,
      7,
      9,
      6,
      3,
      1,
    ],
    by: [
      73,
      25,
      2,
    ],
  },
  "01JUSAGEHELM000000000000": {
    on: [
      26,
      14,
      22,
      12,
      9,
      10,
      3,
      3,
      1,
    ],
    by: [
      83,
      15,
      2,
    ],
  },
};

/** A window's units as a share of the week's, and each Job's. */
const WINDOWS: readonly {
  id: string;
  label: string;
  of: number;
  shares: Shares;
  jobs: readonly number[];
}[] = [
  {
    id: "day",
    label: "24 hours",
    of: 1 / 6.5,
    shares: DAY,
    jobs: [
      0.12,
      0.2,
      0.05,
      0.18,
      0.1,
      0.22,
    ],
  },
  {
    id: "week",
    label: "7 days",
    of: 1,
    shares: WEEK,
    jobs: [
      1,
      1,
      1,
      1,
      1,
      1,
    ],
  },
  {
    id: "fortnight",
    label: "14 days",
    of: 1.9,
    shares: FORTNIGHT,
    jobs: [
      1.6,
      1.9,
      1.2,
      1.8,
      1.5,
      1.3,
    ],
  },
];

/** What the reweighed week was spent on, in three windows. */
export function usageSpentNow(tzOffsetMinutes = 0): UsageSpent {
  const week = reweighedSummaryNow(tzOffsetMinutes);
  const units = new Map(
    week.sessions.map((one) => [
      one.session_id,
      one.usage.units,
    ]),
  );
  const windows: SpentWindow[] = WINDOWS.map((one) => ({
    id: one.id,
    label: one.label,
    total: splitOf(week.total.units * one.of, one.shares),
    jobs: JOBS.map((job, at) => ({
      id: job.id,
      title: job.title,
      split: splitOf((units.get(job.id) ?? 0) * one.jobs[at]!, JOB_SHARES[job.id]!),
    })).sort(
      (a, b) =>
        Object.values(b.split.on).reduce((x, y) => x + y, 0) -
        Object.values(a.split.on).reduce((x, y) => x + y, 0),
    ),
  }));
  return {
    windows,
  };
}

const SINCE_DAYS: Readonly<Record<string, number>> = {
  day: 1,
  week: 7,
  fortnight: 14,
};

/** The same windows as `get_usage_spent` answers them: no titles, each window its span up to now. */
export function wireSpentNow(now = Date.now()): UsageSpentUnits {
  const until = new Date(now).toISOString();
  return {
    until,
    windows: usageSpentNow().windows.map((one) => ({
      id: one.id,
      since: new Date(now - (SINCE_DAYS[one.id] ?? 7) * 86_400_000).toISOString(),
      until,
      total: one.total,
      jobs: one.jobs.map((job) => ({
        session_id: job.id,
        split: job.split,
      })),
    })),
  };
}
