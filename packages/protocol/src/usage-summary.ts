// What a window of turns cost, summed for the Usage page, mirrored by hand from
// `crates/ipc/src/usage_summary.rs`. Burn is in cost
// units: input 1, cache-creation 1.25, cache-read 0.1, output 5, divided by a million. Names and
// counts only.

/** Units under a name: a model, a tool or a `bash_use` label. */
export type NamedUnits = { name: string; units: number };

/** The four token kinds, in units. */
export type TokenUnits = {
  input: number;
  cache_creation: number;
  cache_read: number;
  output: number;
};

/** One local day. `by_model` is left out on the prior period's two days. */
export type DayUnits = {
  /** `YYYY-MM-DD` in the caller's local time. */
  day: string;
  units: number;
  by_model?: NamedUnits[];
};

/** One slice of the turns: the whole window, one Session, or the Sessions past the list. */
export type UsageBreakdown = {
  units: number;
  /** `units` less this is the main sessions' own. */
  subagent_units: number;
  /** Units from turns whose context (input + cache-creation + cache-read) is above 200k. */
  above_200k_units: number;
  /** Above 300k, which is inside the first. */
  above_300k_units: number;
  by_token: TokenUnits;
  by_model: NamedUnits[];
  /** By the first tool a turn called; `none` where it called nothing. */
  by_first_tool: NamedUnits[];
  /** Turns whose first tool was a shell command, by what it was for. */
  by_bash_use: NamedUnits[];
  /** Oldest first; a day with no turn is left out. */
  by_day: DayUnits[];
  /** Seven rows, Monday first, each of eight three-hour blocks from local midnight. */
  hour_grid: number[][];
};

export type SessionUsageSummary = {
  session_id: string;
  /** The most input + cache-creation + cache-read tokens any one turn held. */
  peak_context: number;
  /** Share of its units that subagents spent, 0 to 1. */
  subagent_share: number;
  /** The models it used, most units first. */
  models: string[];
  usage: UsageBreakdown;
};

/** The Sessions past the listed ones, rolled into one. */
export type RestUsage = { sessions: number; usage: UsageBreakdown };

/** The period of equal length just before `since`. */
export type PriorUsage = {
  since: string;
  until: string;
  units: number;
  subagent_share: number;
  above_200k_share: number;
  /** The last whole local day before the window's `until`, and the one before it. */
  last_day: DayUnits;
  day_before: DayUnits;
};

/** `get_usage_summary` (`GET /usage/summary?since=&until=&tz_offset_minutes=`). */
export type UsageSummary = {
  /** The window as resolved, RFC3339 UTC. */
  since: string;
  until: string;
  tz_offset_minutes: number;
  total: UsageBreakdown;
  /** The largest thirty by units, largest first. */
  sessions: SessionUsageSummary[];
  /** Absent where there are no more than thirty. */
  rest?: RestUsage;
  prior: PriorUsage;
};
