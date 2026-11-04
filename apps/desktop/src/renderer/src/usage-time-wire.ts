// Fleet's account of where Jobs' time went, turned into the types the Usage page and a Job's System tab draw. The one place
// the units change: a split is milliseconds on the wire and seconds on the page, and nothing else is converted. A figure Fleet
// left out stays absent all the way down, so a line breaks there and a chip draws a dash, never a 0 and never a NaN.

import type {
  Build,
  TimeJob,
  TimeSplit,
  UsageBuilds,
  UsageHour,
  UsageHours,
  UsageTime,
} from "@armada/components";
import type {
  BuildFigures,
  HourFigures,
  JobTime,
  RecordSplit,
  UsageBreakdown,
  UsageHours as WireHours,
  UsageTime as WireTime,
} from "@armada/protocol";

const HOUR_MS = 3_600_000;
/** The hours the page draws, which is also Fleet's default. */
const SHOWN = 48;

/** A split in milliseconds as seconds, the page's unit. */
export function splitOf(split: RecordSplit): TimeSplit {
  return {
    model: split.model / 1000,
    tools: split.tools / 1000,
    checks: split.checks / 1000,
    subagents: split.subagents / 1000,
    slot: split.slot / 1000,
    you: split.you / 1000,
    pull_request: split.pull_request / 1000,
    stopped: split.stopped / 1000,
    idle: split.idle / 1000,
  };
}

const usualOf = (usual: WireTime["usual"]): Record<string, TimeSplit> =>
  Object.fromEntries(
    usual.map((one) => [
      one.workflow,
      splitOf(one.split),
    ]),
  );

/** A Job's title, or its id where Fleet holds none. */
const titled = (id: string, title: string | undefined): string =>
  title === undefined || title === "" ? id : title;

/** The Usage page's Time view: the window's total, the slowest Jobs and the usual of each workflow. */
export function timeOf(wire: WireTime): UsageTime {
  return {
    total: splitOf(wire.total),
    jobs: wire.slowest.map(
      (one): TimeJob => ({
        id: one.session_id,
        title: titled(one.session_id, one.title),
        workflow: one.workflow,
        split: splitOf(one.split),
      }),
    ),
    usual: usualOf(wire.usual),
  };
}

/** What a Job's System tab draws: the Job's split and the usual of its workflow. Absent where Fleet keeps no split for it. */
export function jobTimeOf(
  wire: JobTime,
  title: string | undefined,
):
  | {
      job: TimeJob;
      usual: TimeSplit | undefined;
    }
  | undefined {
  const kept = wire.time;
  if (kept === undefined) return undefined;
  return {
    job: {
      id: wire.session_id,
      title: titled(wire.session_id, title),
      workflow: kept.workflow,
      split: splitOf(kept.spent),
    },
    usual: kept.usual === undefined ? undefined : splitOf(kept.usual.split),
  };
}

/**
 * Fleet leaves an hour out where nothing ran in it, and places the rest by `hour` from `since`. The page wants 48 points, so
 * an hour with no entry is a point with no figures, and its line breaks there.
 */
export function hoursOf(wire: WireHours): UsageHours {
  const start = Date.parse(wire.since);
  const placed = new Map<number, HourFigures>(
    wire.hours.map((one) => [
      one.hour,
      one,
    ]),
  );
  return Array.from(
    {
      length: SHOWN,
    },
    (_, at): UsageHour => {
      const hour = placed.get(at);
      const begins = new Date(start + at * HOUR_MS).toISOString();
      if (hour === undefined)
        return {
          at: begins,
        };
      return {
        at: begins,
        ...(hour.burn === undefined
          ? {}
          : {
              burn: hour.burn,
            }),
        ...(hour.model === undefined
          ? {}
          : {
              model: hour.model,
            }),
        ...(hour.tools === undefined
          ? {}
          : {
              tools: hour.tools,
            }),
        ...(hour.checks === undefined
          ? {}
          : {
              checks: hour.checks,
            }),
        ...(hour.pull_request === undefined
          ? {}
          : {
              pull_request: hour.pull_request,
            }),
      };
    },
  );
}

/** A build is told from every other by its commit and when it began: a commit run again later is a second build. */
export const buildIdOf = (build: Pick<BuildFigures, "commit" | "started">): string =>
  `${build.commit}@${build.started}`;

/**
 * The page reads a build as a preview where it holds branches and as main where it holds none, so a build Fleet cannot place
 * as one or the other would read as main and say so. Those are left out of the list rather than drawn as that claim.
 */
export const isPlaced = (build: BuildFigures): boolean =>
  build.source === "preview"
    ? build.branches.length > 0 && build.main_commit !== undefined
    : build.source === "main" && build.branches.length === 0;

/** Where a build's run is read from and to, for its tokens: it ends where the next began, or with the hours for the one running. */
export function runOf(
  build: BuildFigures,
  hoursUntil: string,
): {
  since: string;
  until: string;
} {
  return {
    since: build.started,
    until: build.until ?? hoursUntil,
  };
}

/**
 * The builds, newest first as the page lists them. A build's `usage` is what the Tokens view shows when it is picked, which
 * Fleet serves apart as a summary over the build's run: `usages` holds those read so far, by `buildIdOf`.
 */
export function buildsOf(
  wire: WireHours,
  usages: ReadonlyMap<string, UsageBreakdown>,
): UsageBuilds {
  return wire.builds
    .filter(isPlaced)
    .map((one): Build => {
      const id = buildIdOf(one);
      const usage = usages.get(id);
      return {
        id,
        // A main build stands on its own commit.
        commit: one.main_commit ?? one.commit,
        started: one.started,
        branches: one.branches,
        turns: one.turns,
        ...(one.burn === undefined
          ? {}
          : {
              burn: one.burn,
            }),
        ...(one.model === undefined
          ? {}
          : {
              model: one.model,
            }),
        ...(one.tools === undefined
          ? {}
          : {
              tools: one.tools,
            }),
        ...(one.checks === undefined
          ? {}
          : {
              checks: one.checks,
            }),
        ...(one.pull_request === undefined
          ? {}
          : {
              pull_request: one.pull_request,
            }),
        jobs: one.jobs,
        ...(usage === undefined
          ? {}
          : {
              usage,
            }),
        split: splitOf(one.split),
      };
    })
    .reverse();
}
