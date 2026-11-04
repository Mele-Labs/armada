// The mock's time figures as Fleet puts them on the wire, so a scenario answers the three reads through the same path a real
// window does and the adapter (`../usage-time-wire.ts`) is the one place the units change. The sketches themselves stay in
// `job-time.ts`, `usage-builds.ts` and `usage-hours.ts`, in the page's seconds.

import type { Build, TimeSplit, UsageBuilds, UsageHours, UsageTime } from "@armada/components";
import type {
  BuildFigures,
  HourFigures,
  JobTime,
  RecordSplit,
  UsageBreakdown,
  UsageHours as WireHours,
  UsageTime as WireTime,
  UsualTime,
} from "@armada/protocol";

const HOUR = 3_600_000;
/** How many kept Jobs each usual is the median of. */
const KEPT = 12;

const millis = (split: TimeSplit): RecordSplit => ({
  model: split.model * 1000,
  tools: split.tools * 1000,
  checks: split.checks * 1000,
  subagents: split.subagents * 1000,
  slot: split.slot * 1000,
  you: split.you * 1000,
  pull_request: split.pull_request * 1000,
  stopped: split.stopped * 1000,
  idle: split.idle * 1000,
});
const seconds = (split: TimeSplit): number =>
  Object.values(split).reduce((sum, one) => sum + one, 0);

const usualOf = (time: UsageTime): UsualTime[] =>
  Object.entries(time.usual)
    .sort(([a], [b]) => a.localeCompare(b))
    .map(([workflow, split]) => ({
      workflow,
      jobs: KEPT,
      split: millis(split),
    }));

export function wireTime(time: UsageTime, now: number): WireTime {
  return {
    since: new Date(now - 7 * 24 * HOUR).toISOString(),
    until: new Date(now).toISOString(),
    jobs: time.jobs.length,
    total: millis(time.total),
    slowest: time.jobs.map((one, at) => {
      const ended = now - (at + 1) * HOUR;
      return {
        session_id: one.id,
        title: one.title,
        workflow: one.workflow,
        began_at: new Date(ended - seconds(one.split) * 1000).toISOString(),
        ended_at: new Date(ended).toISOString(),
        split: millis(one.split),
      };
    }),
    usual: usualOf(time),
  };
}

/** One Job's split as Fleet keeps it; a Job the window does not list has none kept. */
export function wireJobTime(time: UsageTime, sessionId: string, now: number): JobTime {
  const at = time.jobs.findIndex((one) => one.id === sessionId);
  if (at < 0)
    return {
      session_id: sessionId,
    };
  const one = time.jobs[at]!;
  const ended = now - (at + 1) * HOUR;
  const usual = usualOf(time).find((row) => row.workflow === one.workflow);
  return {
    session_id: sessionId,
    time: {
      workflow: one.workflow,
      began_at: new Date(ended - seconds(one.split) * 1000).toISOString(),
      ended_at: new Date(ended).toISOString(),
      spent: millis(one.split),
      ...(usual === undefined
        ? {}
        : {
            usual,
          }),
    },
  };
}

const figuresOf = (hour: UsageHours[number], at: number): HourFigures | undefined => {
  const { burn, model, tools, checks, pull_request } = hour;
  if (
    burn === undefined &&
    model === undefined &&
    tools === undefined &&
    checks === undefined &&
    pull_request === undefined
  )
    return undefined;
  return {
    hour: at,
    at: hour.at,
    ...(burn === undefined
      ? {}
      : {
          burn: [
            ...burn,
          ],
        }),
    ...(model === undefined
      ? {}
      : {
          model,
        }),
    ...(tools === undefined
      ? {}
      : {
          tools,
        }),
    ...(checks === undefined
      ? {}
      : {
          checks,
        }),
    ...(pull_request === undefined
      ? {}
      : {
          pull_request,
        }),
  };
};

/** The build as Fleet counts it. Its `until` is where the next began; the newest is still running. */
const buildFigures = (build: Build, until: string | undefined): BuildFigures => ({
  commit: build.id,
  started: build.started,
  ...(until === undefined
    ? {}
    : {
        until,
      }),
  source: build.branches.length > 0 ? "preview" : "main",
  main_commit: build.commit,
  branches: [
    ...build.branches,
  ],
  turns: build.turns,
  ...(build.burn === undefined
    ? {}
    : {
        burn: build.burn,
      }),
  ...(build.model === undefined
    ? {}
    : {
        model: build.model,
      }),
  ...(build.tools === undefined
    ? {}
    : {
        tools: build.tools,
      }),
  ...(build.checks === undefined
    ? {}
    : {
        checks: build.checks,
      }),
  ...(build.pull_request === undefined
    ? {}
    : {
        pull_request: build.pull_request,
      }),
  jobs: [
    ...build.jobs,
  ],
  split: millis(build.split),
});

/** The 48 hours with an hour left out where nothing ran in it, and the builds oldest first. */
export function wireHours(hours: UsageHours, builds: UsageBuilds): WireHours {
  const oldestFirst = [
    ...builds,
  ].reverse();
  return {
    since: hours[0]!.at,
    until: new Date(Date.parse(hours[hours.length - 1]!.at) + HOUR).toISOString(),
    hours: hours.flatMap((one, at) => figuresOf(one, at) ?? []),
    builds: oldestFirst.map((one, at) => buildFigures(one, oldestFirst[at + 1]?.started)),
  };
}

/** What Fleet sums over a build's run, which the mock holds on the build. */
export function usageOfBuild(builds: UsageBuilds, since: string): UsageBreakdown | undefined {
  return builds.find((one) => one.started === since)?.usage;
}
