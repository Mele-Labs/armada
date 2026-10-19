// What the Usage page draws, worked out from Fleet's summary: the labels, the shares, the seven local days and the day by
// hour map. Pure, so the page itself only lays out.

import type { DayUnits, NamedUnits, SessionUsageSummary, UsageBreakdown, UsageRuleName, UsageRules, UsageSummary } from "@armada/protocol";

export const WEEKDAYS = ["Mon", "Tue", "Wed", "Thu", "Fri", "Sat", "Sun"] as const;
export const HOURS = ["00h", "03h", "06h", "09h", "12h", "15h", "18h", "21h"] as const;
const MONTHS = ["Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec"] as const;

/** Units, one decimal: the unit is millions, said where the figure is. */
export const M = (units: number): string => units.toFixed(1);

/** A percent: under a tenth of one says so rather than rounding to nothing. */
export const pct = (share: number): string => (share > 0 && share < 0.1 ? "<0.1%" : `${share.toFixed(1)}%`);

/** `part` as a percent of `whole`; nothing to divide by is nothing. */
export const shareOf = (part: number, whole: number): number => (whole > 0 ? (part / whole) * 100 : 0);

/** The model family a name belongs to: `claude-sonnet-4-5` and `sonnet` are one. Anything else keeps its own name. */
export function familyOf(model: string): string {
  const lower = model.toLowerCase();
  return ["sonnet", "opus", "haiku"].find((one) => lower.includes(one)) ?? model;
}

const FAMILY_ORDER = ["sonnet", "opus", "haiku"];

/** Units folded by model family and set in the order the chart stacks them, so a model keeps its place. */
export function foldModels(rows: readonly NamedUnits[]): NamedUnits[] {
  const folded = new Map<string, number>();
  for (const one of rows) folded.set(familyOf(one.name), (folded.get(familyOf(one.name)) ?? 0) + one.units);
  const rank = (name: string) => (FAMILY_ORDER.includes(name) ? FAMILY_ORDER.indexOf(name) : FAMILY_ORDER.length);
  return [...folded].map(([name, units]) => ({ name, units })).sort((a, b) => rank(a.name) - rank(b.name) || b.units - a.units);
}

/** A tool as the page names it: no tool for the turns that called none, and an MCP tool without its prefix. */
export function toolLabel(name: string): string {
  if (name === "none") return "no tool";
  const mcp = /^mcp__(.+?)__(.+)$/.exec(name);
  if (mcp === null) return name;
  return mcp[1]!.startsWith("armada-") ? `${mcp[1]!.slice("armada-".length)} ${mcp[2]}` : mcp[2]!;
}

const BASH_LABELS: Record<string, string> = {
  read_search: "Read and search files",
  scripts: "Scripts, python3 and node",
  git: "Git",
  build_test: "Build, test and checks",
  loops_waits: "Loops, waits and Fleet calls",
  forge: "GitHub",
  other: "Other",
};
export const bashLabel = (name: string): string => BASH_LABELS[name] ?? name;

const TOKEN_LABELS: Record<string, string> = { cache_read: "cache read", cache_creation: "cache creation", output: "output", input: "input" };

/** The four token kinds as rows, the biggest first. */
export function tokenRows(usage: UsageBreakdown): NamedUnits[] {
  return Object.entries(usage.by_token)
    .map(([name, units]) => ({ name: TOKEN_LABELS[name] ?? name, units }))
    .sort((a, b) => b.units - a.units);
}

/** One row of a share cut: a name and its percent of the whole burn. */
export type ShareRow = { name: string; share: number };

/** Rows with something in them, as a percent of `whole`, in the order given. */
export function shareRows(rows: readonly NamedUnits[], whole: number, label: (name: string) => string = (name) => name): ShareRow[] {
  return rows.filter((one) => one.units > 0).map((one) => ({ name: label(one.name), share: shareOf(one.units, whole) }));
}

/** `YYYY-MM-DD` moved by whole days. Calendar arithmetic, so a local day is never a 23 or 25 hour affair. */
export function addDays(day: string, days: number): string {
  const at = new Date(`${day}T00:00:00Z`);
  at.setUTCDate(at.getUTCDate() + days);
  return at.toISOString().slice(0, 10);
}

/** Monday is 0. */
export function weekdayOf(day: string): number {
  return (new Date(`${day}T00:00:00Z`).getUTCDay() + 6) % 7;
}

/** `Sat Oct 17`. */
export function dayLabel(day: string): string {
  const [, month, date] = day.split("-");
  return `${WEEKDAYS[weekdayOf(day)]} ${MONTHS[Number(month) - 1]} ${Number(date)}`;
}

/** The local day an instant falls on, `offsetMinutes` ahead of UTC. */
export function localDay(instant: string, offsetMinutes: number): string {
  return new Date(Date.parse(instant) + offsetMinutes * 60_000).toISOString().slice(0, 10);
}

export type ChartDay = { day: string; units: number; byModel: NamedUnits[]; weekday: number };

/**
 * The seven local days ending on the one before the window's `until`, oldest first. Fleet leaves a day with no turn out,
 * so the gaps are filled here with none.
 */
export function chartDays(summary: Pick<UsageSummary, "until" | "tz_offset_minutes">, byDay: readonly DayUnits[]): ChartDay[] {
  const last = localDay(new Date(Date.parse(summary.until) - 1).toISOString(), summary.tz_offset_minutes);
  return Array.from({ length: 7 }, (_, at) => {
    const day = addDays(last, at - 6);
    const found = byDay.find((one) => one.day === day);
    return { day, units: found?.units ?? 0, byModel: foldModels(found?.by_model ?? []), weekday: weekdayOf(day) };
  });
}

/** Fleet's grid is a row per weekday; the map is a row per three-hour block, so the two are turned. */
export function heatOf(grid: readonly (readonly number[])[]): number[][] {
  return HOURS.map((_, block) => WEEKDAYS.map((__, weekday) => grid[weekday]?.[block] ?? 0));
}

/** The weekday column and block of the busiest cell; the first if nothing burned. */
export function peakCell(heat: readonly (readonly number[])[]): { d: number; b: number } {
  let best = { d: 0, b: 0 };
  heat.forEach((row, b) => row.forEach((units, d) => { if (units > heat[best.b]![best.d]!) best = { d, b }; }));
  return best;
}

/** Upper bounds of the first three steps, units per 3 hours; a fourth step is everything above, and none is nothing. */
export const BOUNDS = [25, 50, 75] as const;
export const stepOf = (units: number): number => (units <= 0 ? 0 : units <= BOUNDS[0] ? 1 : units <= BOUNDS[1] ? 2 : units <= BOUNDS[2] ? 3 : 4);

/** The name of an hour block: `21h to 00h`. */
export const blockLabel = (block: number): string => `${HOURS[block]} to ${String((block * 3 + 3) % 24).padStart(2, "0")}h`;

/** The smallest round step that puts an axis topping out at `top` within four ticks. */
export function tickStep(top: number): number {
  const magnitude = 10 ** Math.floor(Math.log10(Math.max(top, 1)));
  const steps = [0.1, 0.2, 0.5, 1, 2, 5, 10].map((one) => one * magnitude);
  return steps.find((one) => Math.ceil(top / one) <= 4) ?? magnitude;
}

/** The last six characters of an id, for a Job the page holds no title for. */
export const shortId = (id: string): string => id.slice(-6);

/** The percent a delta moved, in whole points, and which way; nothing where it did not move or there was nothing before. */
export function pointsDelta(now: number, before: number, hadBefore: boolean): { up: boolean; points: number } | undefined {
  if (!hadBefore) return undefined;
  const points = Math.round(Math.abs(now - before));
  return points === 0 ? undefined : { up: now > before, points };
}

/** The percent a figure moved against what it was, and which way; nothing where there was nothing to move from. */
export function percentDelta(now: number, before: number): { up: boolean; percent: number } | undefined {
  if (before <= 0) return undefined;
  const percent = Math.abs(((now - before) / before) * 100);
  return Number(percent.toFixed(1)) === 0 ? undefined : { up: now > before, percent };
}

/** One Job's row in the list. */
export type Row = { id: string; title: string; known: boolean; units: number; session: SessionUsageSummary };

/** The rows the list draws: Fleet's Jobs, each under the title Bridge holds for it, else its short id. */
export function rowsOf(summary: UsageSummary, titles: ReadonlyMap<string, string> | undefined): Row[] {
  return summary.sessions.map((session) => {
    const title = titles?.get(session.session_id);
    return { id: session.session_id, title: title ?? shortId(session.session_id), known: title !== undefined, units: session.usage.units, session };
  });
}


/** A trip as the rules panel draws it. `after` is absent until Fleet has measured it. */
export type TripView = { id: string; at: string; figure: string; before: number; after: number | undefined };

/** A rule as the rules panel draws it: its words fixed by the rule, its numbers from the limit and the trips. */
export type RuleView = { id: UsageRuleName; name: string; limit: string; limit_tip: string; figure_tip: string; on: boolean; tell: string; tell_figure: string; trips: TripView[] };

type Words = { name: string; limit: (value: number) => string; tip: (value: number) => string; figure: (figure: number) => string; figure_tip: string; tell: string; tell_figure: string };

const WORDS: Record<UsageRuleName, Words> = {
  context: {
    name: "Context",
    limit: (value) => `${value}k`,
    tip: (value) => `Last turn above ${value}k tokens`,
    figure: (figure) => `${Math.round(figure / 1000)}k`,
    figure_tip: "Context of the last turn",
    tell: "Context is {figure} tokens. To carry on in a new job, ask the owner with the options \"Carry on in a new Job\", \"Stay here\", \"Stay, don't ask again this Job\" and \"Remind me at a higher limit\", and write your brief (where the work is, what is open, what you hold) in the description of \"Carry on in a new Job\". Start nothing yourself.",
    tell_figure: "312k",
  },
  subagents: {
    name: "Subagents",
    limit: (value) => `${value}% of the last hour`,
    tip: (value) => `Over ${value}% of the last hour's burn in subagents`,
    figure: (figure) => `${Math.round(figure)}%`,
    figure_tip: "Share of the last hour's burn that ran in subagents",
    tell: "Subagents burned {figure} of your last hour, on opus. Pass model sonnet on reading and searching agents.",
    tell_figure: "71%",
  },
  cache_hit: {
    name: "Cache",
    limit: (value) => `under ${value}% over 10 turns`,
    tip: (value) => `Cache hit rate under ${value}% over the last 10 turns`,
    figure: (figure) => `${Math.round(figure)}%`,
    figure_tip: "Cache hit rate over the last 10 turns",
    tell: "Cache hit rate is {figure} over your last 10 turns. Something in the prompt or tools changed each turn; check hooks and MCP servers.",
    tell_figure: "54%",
  },
  opus_reading: {
    name: "Opus reading",
    limit: (value) => `${value} turns only reading`,
    tip: (value) => `The last ${value} turns on opus only read and searched`,
    figure: (figure) => `${figure}`,
    figure_tip: "Turns on opus in a row that only read and searched",
    tell: "Your last 20 turns on opus only read and searched. Subagents for reading can run on sonnet.",
    tell_figure: "",
  },
};

/** The rules Fleet answered, in its order, each worded for the panel. A rule the page has no words for is left out. */
export function ruleViews(answer: UsageRules): RuleView[] {
  return answer.rules.flatMap((one) => {
    const words = WORDS[one.rule];
    if (words === undefined) return [];
    const value = one.limit.value;
    return [
      {
        id: one.rule,
        name: words.name,
        limit: words.limit(value),
        limit_tip: words.tip(value),
        figure_tip: words.figure_tip,
        on: one.on,
        tell: words.tell,
        tell_figure: words.tell_figure,
        trips: one.trips.map((trip) => ({ id: trip.session_id, at: trip.at, figure: words.figure(trip.figure), before: trip.burn_per_hour_before, after: trip.burn_per_hour_after })),
      },
    ];
  });
}
