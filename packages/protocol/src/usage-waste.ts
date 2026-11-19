// The token waste Fleet can name, with its cost and the Jobs it hit, mirrored by hand from `crates/ipc/src/usage_waste.rs`.
// Units weighed as the usage summary weighs them. No unit is in two kinds, so the kinds add up to `waste_units`. `title`
// and `fix` are Fleet's sentences. Names and counts only: a Job is its id, and the one input named is the file a Read touched, in `causes`.

/** The window the waste was read over: `day`, `week` or `fortnight`. */
export type WasteWindow = {
  id: string;
  since: string;
  until: string;
};

/** One Job's share of a kind. */
export type WasteJob = {
  session_id: string;
  units: number;
  occurrences: number;
  /** The most context one of the counted requests held; carried by the two context kinds only. */
  peak_context?: number;
  /** The three largest causes, largest first: files for `repeat_reads`, tools for `big_results`. Absent for every other kind. */
  causes?: WasteCause[];
};

/** A file or a tool that a Job's share of a kind came of, and what it cost. */
export type WasteCause = {
  name: string;
  units: number;
};

/** One kind of waste: `subagent_context`, `main_context`, `cold_cache`, `fresh_starts`, `repeat_reads` or `big_results`. */
export type WasteKind = {
  kind: string;
  title: string;
  fix: string;
  units: number;
  occurrences: number;
  /** The thirty largest Jobs, largest first. */
  jobs: WasteJob[];
};

/** `get_usage_waste` (`GET /usage/waste?window=`). */
export type UsageWaste = {
  until: string;
  window: WasteWindow;
  total_units: number;
  waste_units: number;
  /** Largest first; a kind with nothing is left out. */
  kinds: WasteKind[];
};
