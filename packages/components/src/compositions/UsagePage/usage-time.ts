// Where a Job's time went, as the Usage page's Time view and a Job's own split draw it. These types are the
// page's own: the window turns Fleet's reading into them (`usage-time-wire.ts`). Seconds throughout; every second of a Job lands in one bucket.

export const BUCKETS = [
  "model",
  "tools",
  "checks",
  "subagents",
  "slot",
  "you",
  "pull_request",
  "stopped",
  "idle",
] as const;
export type Bucket = (typeof BUCKETS)[number];

/** Seconds a Job spent in each bucket. */
export type TimeSplit = Readonly<Record<Bucket, number>>;

export const BUCKET_LABEL: Readonly<Record<Bucket, string>> = {
  model: "Model",
  tools: "Tools",
  checks: "Checks",
  subagents: "Subagents",
  slot: "Slot",
  you: "You",
  pull_request: "Pull request",
  stopped: "Stopped",
  idle: "Idle",
};

/** What each bucket means: the tooltip on its name wherever the name is drawn. */
export const BUCKET_MEANS: Readonly<Record<Bucket, string>> = {
  model: "Waiting on the model's answer",
  tools: "A tool ran in the main thread",
  checks: "A Check queued or ran",
  subagents: "The main thread waited on a subagent",
  slot: "Waiting for a slot, or for Fleet to move the Job into one",
  you: "Waiting on the owner: a question, a walk, a plan, a start card",
  pull_request: "CI and the merge queue, with nothing else running",
  stopped: "The Job's agent was not running, or Fleet was down, and nothing was asked",
  idle: "Nothing running and nothing asked",
};

/** One Job: its split, and the workflow whose usual it is read against. */
export type TimeJob = {
  id: string;
  title: string;
  workflow: string;
  split: TimeSplit;
};

export type UsageTime = {
  /** Every second of the window, by bucket. */
  total: TimeSplit;
  /** Slowest first. */
  jobs: readonly TimeJob[];
  /** The usual Job of each workflow: the median seconds in each bucket. */
  usual: Readonly<Record<string, TimeSplit>>;
};

export const sumOf = (split: TimeSplit, among: readonly Bucket[] = BUCKETS): number =>
  among.reduce((sum, one) => sum + split[one], 0);

/** Buckets by time spent, most first; a bucket with no time is left out. */
export const bySpent = (split: TimeSplit): Bucket[] =>
  BUCKETS.filter((one) => split[one] > 0).sort(
    (a, b) => split[b] - split[a] || BUCKETS.indexOf(a) - BUCKETS.indexOf(b),
  );

/** A bucket is far over its usual past twice it and at least ten minutes beyond it. */
const FAR = 2;
const MARGIN = 10 * 60;
export const farOver = (spent: number, usual: number | undefined): boolean =>
  usual !== undefined && spent >= usual * FAR && spent - usual >= MARGIN;

/** Seconds as `2h 05m`, `48m` or `<1m`; hours drop the minutes only when they are none. */
export function span(seconds: number): string {
  const minutes = Math.round(seconds / 60);
  if (seconds > 0 && minutes < 1) return "<1m";
  if (minutes < 60) return `${minutes}m`;
  const rest = minutes % 60;
  return rest === 0
    ? `${Math.floor(minutes / 60)}h`
    : `${Math.floor(minutes / 60)}h ${String(rest).padStart(2, "0")}m`;
}

/** The same in a sentence for a reader: `2 hours 5 minutes`. */
export function spanSaid(seconds: number): string {
  const minutes = Math.round(seconds / 60);
  if (minutes < 1) return seconds > 0 ? "under a minute" : "none";
  const hours = Math.floor(minutes / 60);
  const rest = minutes % 60;
  const part = (n: number, unit: string) => `${n} ${unit}${n === 1 ? "" : "s"}`;
  return [
    ...(hours > 0 ? [part(hours, "hour")] : []),
    ...(rest > 0 ? [part(rest, "minute")] : []),
  ].join(" ");
}

const MONTHS = [
  "Jan",
  "Feb",
  "Mar",
  "Apr",
  "May",
  "Jun",
  "Jul",
  "Aug",
  "Sep",
  "Oct",
  "Nov",
  "Dec",
] as const;

/** A date as `Nov 2`. */
export const dayName = (day: string): string => `${MONTHS[Number(day.slice(5, 7)) - 1]} ${Number(day.slice(8))}`;
