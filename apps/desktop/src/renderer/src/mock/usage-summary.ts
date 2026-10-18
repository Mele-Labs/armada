// The Usage page's Fleet: a usage summary shaped from the measured baseline of 11 to 17 Oct, laid on the last seven days
// from today. The week's totals, day totals, splits and hour grid are measured. Each Job's own breakdown (its days, tools,
// Bash use, tokens, hours) is a sketch scaled from the week's, since the baseline held only a Job's units, subagent share,
// peak context, models and model split. The prior period is a sketch. Mock only: Fleet serves the real one.

import type { NamedUnits, SessionUsageSummary, UsageBreakdown, UsageSummary } from "@armada/protocol";

const DAY_MS = 24 * 60 * 60 * 1000;

/**
 * Measured, units in millions, by weekday, Monday first: Mon 12 to Sat 17 Oct and the partial Sunday 11. The summary lays
 * each on the weekday it falls on in the last seven days, so the hour grid, which is by weekday, always agrees with the days.
 */
const BURN_BY_WEEKDAY = [260.8, 285.1, 264.9, 184.2, 305.0, 292.3, 69.9];

/** Measured: units by 3 hour block (rows) and weekday, Monday first (columns). Sunday is partial. */
const HEAT_BY_BLOCK = [
  [50.6, 39.3, 37.2, 5.6, 0.2, 37.4, 0.0],
  [3.3, 0.0, 0.0, 0.0, 0.0, 52.7, 0.0],
  [2.4, 10.3, 15.5, 11.2, 5.5, 72.8, 0.0],
  [79.4, 69.9, 13.7, 29.4, 63.4, 58.6, 0.0],
  [16.6, 84.0, 18.5, 8.3, 54.7, 32.5, 0.0],
  [0.0, 23.6, 53.8, 52.5, 47.2, 14.5, 0.0],
  [0.0, 1.9, 38.7, 8.3, 26.4, 0.0, 0.0],
  [103.7, 51.5, 82.6, 65.6, 107.6, 34.9, 69.9],
];
const HEAT_GRID = [0, 1, 2, 3, 4, 5, 6].map((weekday) => HEAT_BY_BLOCK.map((row) => row[weekday]!));

type Share = { name: string; share: number };

const WEEK_MODELS: Share[] = [{ name: "sonnet", share: 76.3 }, { name: "opus", share: 23.5 }, { name: "haiku", share: 0.2 }];
const WEEK_TOOLS: Share[] = [
  { name: "Bash", share: 79.0 }, { name: "none", share: 7.6 }, { name: "Read", share: 3.5 }, { name: "Write", share: 2.1 },
  { name: "SendMessage", share: 1.3 }, { name: "Edit", share: 1.1 }, { name: "ToolSearch", share: 0.8 }, { name: "mcp__armada-fleet__waiting_for", share: 0.8 },
  { name: "Agent", share: 0.8 }, { name: "Monitor", share: 0.5 }, { name: "Skill", share: 0.4 }, { name: "mcp__armada-fleet__show_window", share: 0.4 },
  { name: "AskUserQuestion", share: 0.2 }, { name: "TaskUpdate", share: 0.2 },
];
const WEEK_BASH: Share[] = [
  { name: "read_search", share: 42.2 }, { name: "scripts", share: 14.9 }, { name: "git", share: 7.1 }, { name: "build_test", share: 4.7 },
  { name: "loops_waits", share: 3.6 }, { name: "forge", share: 2.2 }, { name: "other", share: 4.3 },
];
const WEEK_TOKENS: Share[] = [{ name: "cache_read", share: 69.8 }, { name: "cache_creation", share: 18.4 }, { name: "output", share: 11.8 }, { name: "input", share: 0.04 }];

const sum = (rows: readonly Share[]) => rows.reduce((a, b) => a + b.share, 0);
const units = (rows: readonly Share[], total: number): NamedUnits[] => rows.map((one) => ({ name: one.name, units: (one.share * total) / sum(rows) })).sort((a, b) => b.units - a.units);
/** A Job's own split is the week's, nudged a fixed amount per Job so each reads differently. */
const nudge = (rows: readonly Share[], seed: number): Share[] => rows.map((one, i) => ({ name: one.name, share: one.share * (1 + (((seed * 7 + i * 5) % 9) - 4) / 10) }));

const weekdayOf = (day: string) => (new Date(`${day}T00:00:00Z`).getUTCDay() + 6) % 7;
const dayOf = (at: number, offsetMinutes: number) => new Date(at + offsetMinutes * 60_000).toISOString().slice(0, 10);

/** `days` are weights by weekday, Monday first. */
type Job = { id: string; title: string; units: number; subagent: number; peak: number; models: string[]; byModel: Share[]; above200: number; days: number[] };

/** The Jobs of the baseline: units, subagent share, peak context, models and model split measured; their days a sketch. */
export const JOBS: Job[] = [
  { id: "01JUSAGEMERGER0000000000", title: "Jobs -> Session Merger", units: 344.6, subagent: 87, peak: 372_000, models: ["claude-opus-4", "claude-sonnet-4"], byModel: [{ name: "sonnet", share: 81.2 }, { name: "opus", share: 18.6 }, { name: "haiku", share: 0.2 }], above200: 61, days: [10, 24, 30, 12, 16, 8, 0] },
  { id: "01JUSAGECOCKPIT000000000", title: "Cockpit Updates", units: 166.0, subagent: 85, peak: 368_000, models: ["claude-opus-4", "claude-sonnet-4"], byModel: [{ name: "sonnet", share: 74.5 }, { name: "opus", share: 25.4 }, { name: "haiku", share: 0.1 }], above200: 58, days: [18, 14, 10, 6, 28, 18, 6] },
  { id: "01JUSAGEDEBUG00000000000", title: "Debug Job 14", units: 116.7, subagent: 66, peak: 962_000, models: ["claude-opus-4"], byModel: [{ name: "opus", share: 71.0 }, { name: "sonnet", share: 28.8 }, { name: "haiku", share: 0.2 }], above200: 79, days: [0, 8, 30, 22, 12, 0, 0] },
  { id: "01JUSAGEMERGELINE0000000", title: "Multi Repo Merge Line and PR Monitoring", units: 72.4, subagent: 77, peak: 367_000, models: ["claude-sonnet-4"], byModel: [{ name: "sonnet", share: 93.0 }, { name: "opus", share: 6.9 }, { name: "haiku", share: 0.1 }], above200: 47, days: [12, 10, 8, 14, 10, 6, 4] },
  { id: "01JUSAGESYNCING000000000", title: "Syncing Armada between Machines", units: 70.2, subagent: 73, peak: 368_000, models: ["claude-sonnet-4"], byModel: [{ name: "sonnet", share: 88.4 }, { name: "opus", share: 11.5 }, { name: "haiku", share: 0.1 }], above200: 49, days: [2, 6, 14, 16, 12, 10, 0] },
  { id: "01JUSAGEHELM000000000000", title: "Helm as a distributor", units: 68.9, subagent: 71, peak: 616_000, models: ["claude-opus-4", "claude-sonnet-4"], byModel: [{ name: "opus", share: 52.3 }, { name: "sonnet", share: 47.5 }, { name: "haiku", share: 0.2 }], above200: 66, days: [0, 0, 6, 12, 22, 20, 0] },
];

/** The titles a Bridge holds for these Jobs, as its sessions list would. */
export const JOB_TITLES: ReadonlyMap<string, string> = new Map(JOBS.map((one) => [one.id, one.title]));

function breakdown(opts: {
  total: number;
  subagent: number;
  above200: number;
  models: Share[];
  tools: Share[];
  bash: Share[];
  tokens: Share[];
  dayUnits: number[];
  grid: number[][];
  days: string[];
}): UsageBreakdown {
  const { total, days } = opts;
  const byDay = days
    .map((day, at) => ({ day, units: opts.dayUnits[at]!, by_model: units(opts.models, opts.dayUnits[at]!) }))
    .filter((one) => one.units > 0);
  return {
    units: total,
    subagent_units: (total * opts.subagent) / 100,
    above_200k_units: (total * opts.above200) / 100,
    above_300k_units: (total * opts.above200 * 0.46) / 100,
    by_token: Object.fromEntries(units(opts.tokens, total).map((one) => [one.name, one.units])) as UsageBreakdown["by_token"],
    by_model: units(opts.models, total),
    by_first_tool: units(opts.tools, total),
    by_bash_use: units(opts.bash, (total * opts.tools[0]!.share) / sum(opts.tools)),
    by_day: byDay,
    hour_grid: opts.grid,
  };
}

function jobUsage(job: Job, seed: number, days: string[]): SessionUsageSummary {
  const tools = nudge(WEEK_TOOLS, seed);
  const weights = days.map((day) => job.days[weekdayOf(day)]!);
  const dayUnits = weights.map((weight) => (job.units * weight) / weights.reduce((a, b) => a + b, 0));
  // Its hours: the week's grid scaled to its own days, laid on the weekday each day falls on.
  const grid = HEAT_GRID.map((column) => column.map(() => 0));
  days.forEach((day, at) => {
    const wd = weekdayOf(day);
    const column = HEAT_GRID[wd]!;
    grid[wd] = column.map((cell) => (cell / column.reduce((a, b) => a + b, 0)) * dayUnits[at]!);
  });
  return {
    session_id: job.id,
    peak_context: job.peak,
    subagent_share: job.subagent / 100,
    models: job.models,
    usage: breakdown({ total: job.units, subagent: job.subagent, above200: job.above200, models: job.byModel, tools, bash: nudge(WEEK_BASH, seed + 3), tokens: nudge(WEEK_TOKENS, seed + 5), dayUnits, grid, days }),
  };
}

/** What Fleet would answer for the last seven days, a caller `tzOffsetMinutes` ahead of UTC. */
export function usageSummaryNow(tzOffsetMinutes: number, now = Date.now()): UsageSummary {
  const days = Array.from({ length: 7 }, (_, at) => dayOf(now - (6 - at) * DAY_MS, tzOffsetMinutes));
  const dayUnits = days.map((day) => BURN_BY_WEEKDAY[weekdayOf(day)]!);
  const total = dayUnits.reduce((a, b) => a + b, 0);
  const sessions = JOBS.map((job, at) => jobUsage(job, at + 1, days));
  const prior = { since: new Date(now - 14 * DAY_MS).toISOString(), until: new Date(now - 7 * DAY_MS).toISOString() };
  return {
    since: new Date(now - 7 * DAY_MS).toISOString(),
    until: new Date(now).toISOString(),
    tz_offset_minutes: tzOffsetMinutes,
    total: breakdown({ total, subagent: 70, above200: 56, models: WEEK_MODELS, tools: WEEK_TOOLS, bash: WEEK_BASH, tokens: WEEK_TOKENS, dayUnits, grid: HEAT_GRID, days }),
    sessions,
    prior: {
      ...prior,
      units: 1500,
      subagent_share: 0.66,
      above_200k_share: 0.61,
      last_day: { day: days[5]!, units: dayUnits[5]! },
      day_before: { day: days[4]!, units: dayUnits[4]! },
    },
  };
}
