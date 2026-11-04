// Where Jobs' time went, mirrored by hand from `crates/ipc/src/time_summary.rs`. A split is `RecordSplit`,
// milliseconds, wherever it appears; the four medians of an hour or a build are seconds. A figure nobody
// produced is absent, never 0. Names and counts only.

import type { NamedUnits } from "./usage-summary";
import type { RecordSplit } from "./retro";

/** The usual Job of a workflow: the median milliseconds in each bucket over every kept Job of it. */
export type UsualTime = {
  workflow: string;
  /** How many kept Jobs of the workflow the medians are of. */
  jobs: number;
  split: RecordSplit;
};

/** A Job's whole time, kept when it ended. */
export type JobSplit = {
  /** Empty where the Job ran no named workflow. */
  workflow: string;
  began_at: string;
  ended_at: string;
  /** The buckets sum to `ended_at` less `began_at`. */
  spent: RecordSplit;
  /** Absent where the Job has no workflow name. */
  usual?: UsualTime;
};

/** `get_job_time` (`GET /sessions/time?session_id=`): one Job's split against the usual of its workflow. */
export type JobTime = {
  session_id: string;
  /** Absent where none is kept: the Job is still running, or ended before splits were kept. */
  time?: JobSplit;
};

/** One of the longest Jobs of a window. */
export type SlowJob = {
  session_id: string;
  /** Absent where Fleet holds no title for the Job. */
  title?: string;
  workflow: string;
  began_at: string;
  ended_at: string;
  split: RecordSplit;
};

/** `get_usage_time` (`GET /usage/time?since=&until=&slowest=`): the Jobs that ended in `[since, until)`. */
export type UsageTime = {
  /** The window as resolved, RFC3339 UTC. */
  since: string;
  until: string;
  /** How many Jobs ended in it. */
  jobs: number;
  /** Every Job's split, summed. */
  total: RecordSplit;
  /** The longest first. */
  slowest: SlowJob[];
  /** By workflow, alphabetically, from every Job kept of it and not only this window's. */
  usual: UsualTime[];
};

/**
 * One hour that had something in it. The four medians are seconds, each absent where the hour produced
 * none, so a line breaks there and does not fall to zero.
 */
export type HourFigures = {
  /** Hours from the window's `since`, from 0. */
  hour: number;
  /** The hour's start, RFC3339 UTC. */
  at: string;
  /** Cost units by model, most first, in millions; absent where no turn was taken in the hour. */
  burn?: NamedUnits[];
  /** The wait on the model per turn of the main thread. */
  model?: number;
  /** The time the tools of a turn ran, per turn that called any. */
  tools?: number;
  /** A Check run's time, queued or running. */
  checks?: number;
  /** A pull request's wait in CI and the merge queue with nothing else running. */
  pull_request?: number;
};

/** One build Fleet ran and what ran on it. The four medians are as on `HourFigures`, over the build's whole run. */
export type BuildFigures = {
  /** The build's own commit, its first seven characters. A commit run twice has two builds, told apart by `started`. */
  commit: string;
  started: string;
  /** When the next build began, which is where this one's run ends; absent while it runs. */
  until?: string;
  /** Absent where git had not said. */
  source?: "main" | "preview";
  /** The main commit under it, seven characters; absent where git had not said. */
  main_commit?: string;
  /** What a preview holds; empty for a build on main. */
  branches: string[];
  /** Every turn kept while it ran, subagents' included. */
  turns: number;
  /** Cost units per turn, not millions of them. Absent where the build took no turn. */
  burn?: number;
  model?: number;
  tools?: number;
  checks?: number;
  pull_request?: number;
  /** The Jobs that took a turn on it. */
  jobs: string[];
  /** The time of every Job in the stretch it ran, cut at its start. */
  split: RecordSplit;
};

/** `get_usage_hours` (`GET /usage/hours?since=&hours=`): the hours from `since`, and the builds that ran in them. */
export type UsageHours = {
  /** The first hour's start and the end of the last, RFC3339 UTC. */
  since: string;
  until: string;
  /** Oldest first. An hour in which nothing ran is left out: place each by `hour`. */
  hours: HourFigures[];
  /** The builds that ran at any time since `since`, the one running at `since` among them, oldest first. */
  builds: BuildFigures[];
};
