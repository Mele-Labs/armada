// One Job's burn over its life, mirrored by hand from `crates/ipc/src/usage_job_timeline.rs`.
// Units weighed as the usage summary weighs them. Every stamp is RFC3339. `label` is Fleet's words for what a pause
// waited on, so a page and an agent read the same ones. Names and counts only.

/** The burn of one bucket of time, split by who spent it. A compaction's own request is the main thread's. */
export type TimelineBucket = {
  /** Where the bucket begins. */
  at: string;
  main_units: number;
  subagent_units: number;
};

/** The main thread's context at one request: its input, cache writes and cache read, in tokens. */
export type TimelineContext = {
  at: string;
  tokens: number;
};

/** A gap of more than five minutes between two main-thread requests. */
export type TimelinePause = {
  from: string;
  to: string;
  /** `you`, `command`, `subagent` or `other`. */
  waited_on: string;
  /** Fleet's words for `waited_on`: `Waited on you`, `Command ran`, `Subagent ran` or `Paused`. */
  label: string;
  /** The request after the pause found the cache cold. */
  cold: boolean;
};

/** `get_job_timeline` (`GET /usage/job_timeline?session_id=`). */
export type JobTimeline = {
  session_id: string;
  started: string;
  /** Absent while the Job runs. */
  ended?: string;
  /** 60, 300, 600, 900, 1800 or 3600. */
  bucket_seconds: number;
  total_units: number;
  /** Every Job's burn over the seven days ending at `ended`, or now. */
  week_units: number;
  /** Every bucket from `started` to the end of the span, oldest first, one with no burn included. */
  buckets: TimelineBucket[];
  /** The main thread, a point per request, thinned to at most 400. */
  context: TimelineContext[];
  compactions: string[];
  pauses: TimelinePause[];
  peak_context: number;
  cold_wakes: number;
  /** The part of `total_units` subagents burned, 0 to 1. */
  subagent_share: number;
};
