// What the burn was spent on, and who spent it, mirrored by hand from `crates/ipc/src/usage_spent.rs`. Units weighed as
// the usage summary weighs them, so a window's whole is the summary's for the same span; both splits add up to it. Names
// and counts only: a Job is its id.

/** Units by what they went on. `docs/concepts/session/where-the-tokens-went.md` names each part. */
export type SpentOnUnits = {
  start_up: number;
  reads: number;
  own_words: number;
  output: number;
  cold_cache: number;
  announced: number;
  other_results: number;
  messages: number;
  background: number;
};

/** Units by who spent them: `subagents` is the summary's `subagent_units`. */
export type SpentByUnits = {
  main: number;
  subagents: number;
  background: number;
};

export type SpentSplitUnits = {
  on: SpentOnUnits;
  by: SpentByUnits;
};

export type SpentJobUnits = {
  session_id: string;
  split: SpentSplitUnits;
};

/** One window up to `until`: `day`, `week` or `fortnight`. */
export type SpentWindowUnits = {
  id: string;
  since: string;
  until: string;
  total: SpentSplitUnits;
  /** The thirty largest Jobs by units, largest first. */
  jobs: SpentJobUnits[];
};

/** `get_usage_spent` (`GET /usage/spent?until=`). */
export type UsageSpentUnits = {
  until: string;
  /** Shortest first. */
  windows: SpentWindowUnits[];
};
