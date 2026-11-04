// Where the time went, for the Usage page's Time view and a Job's System tab. Mock only: these are
// sketches, put on the wire by `usage-time-wire.ts`. The Jobs are the Usage week's own, so a Job's row here is the one its System tab reads.

import type { TimeJob, TimeSplit, UsageTime } from "@armada/components";

import { JOBS } from "./usage-summary";

const MINUTE = 60;

/** A split from minutes, in the buckets' own order: model, tools, checks, subagents, slot, you, pull request, stopped, idle. */
const minutes = (
  model: number,
  tools: number,
  checks: number,
  subagents: number,
  slot: number,
  you: number,
  pullRequest: number,
  stopped: number,
  idle: number,
): TimeSplit => ({
  model: model * MINUTE,
  tools: tools * MINUTE,
  checks: checks * MINUTE,
  subagents: subagents * MINUTE,
  slot: slot * MINUTE,
  you: you * MINUTE,
  pull_request: pullRequest * MINUTE,
  stopped: stopped * MINUTE,
  idle: idle * MINUTE,
});

const FEATURE = "Build a feature";
const FIX = "Fix a bug";
const CLEANUP = "Clean up";

const USUAL = {
  [FEATURE]: minutes(40, 14, 12, 12, 4, 25, 12, 5, 10),
  [FIX]: minutes(22, 9, 8, 4, 3, 14, 10, 4, 8),
  [CLEANUP]: minutes(18, 12, 10, 2, 2, 6, 8, 2, 5),
};

const title = (id: string): string => JOBS.find((one) => one.id === id)!.title;
const job = (id: string, workflow: string, split: TimeSplit, named?: string): TimeJob => ({
  id,
  title: named ?? title(id),
  workflow,
  split,
});

/** Three slow Jobs carry hours of Stopped, one of them nearly all of it; Checks and Pull request run far past the usual in the two after it. */
const JOBS_BY_TIME: TimeJob[] = [
  job("01JUSAGEMERGER0000000000", FEATURE, minutes(62, 22, 148, 20, 9, 64, 121, 190, 12)),
  job("01JUSAGEMERGELINE0000000", FEATURE, minutes(71, 20, 96, 18, 31, 40, 88, 140, 14)),
  job("01JUSAGECOCKPIT000000000", FEATURE, minutes(58, 19, 54, 11, 6, 51, 62, 0, 17)),
  job("01JUSAGEDEBUG00000000000", FIX, minutes(45, 18, 61, 6, 5, 22, 43, 95, 13)),
  job("01JUSAGESYNCING000000000", FEATURE, minutes(18, 6, 12, 4, 2, 8, 14, 1080, 6)),
  job("01JUSAGEHELM000000000000", FEATURE, minutes(39, 12, 29, 10, 3, 28, 24, 0, 9)),
  job(
    "01JUSAGETOOLTIP00000000",
    CLEANUP,
    minutes(17, 11, 33, 1, 2, 4, 29, 0, 3),
    "Rename the tooltip helper",
  ),
  job(
    "01JUSAGEBRANCHLABEL00000",
    FIX,
    minutes(21, 8, 22, 3, 2, 10, 19, 0, 6),
    "Fix the stale branch label",
  ),
];
const sumOf = (split: TimeSplit): number => Object.values(split).reduce((sum, one) => sum + one, 0);
/** Slowest first, by every bucket. */
const SLOWEST = [
  ...JOBS_BY_TIME,
].sort((a, b) => sumOf(b.split) - sumOf(a.split));

/** Every second of the window: the Jobs above and the many like them. */
const total = (): TimeSplit => minutes(2900, 1050, 3400, 900, 480, 2300, 2700, 1900, 800);

export function jobTimeNow(): UsageTime {
  return {
    total: total(),
    jobs: SLOWEST,
    usual: USUAL,
  };
}
