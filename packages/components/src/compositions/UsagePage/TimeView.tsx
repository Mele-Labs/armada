import { useEffect, useRef, useState } from "react";

import { Sheet } from "../../primitives/Sheet/Sheet";
import { Skeleton } from "../../primitives/Skeleton/Skeleton";
import { Tooltip } from "../../primitives/Tooltip/Tooltip";
import { BuildsPanel } from "./UsageBuilds";
import { HourChart, HourRows, PendingHours } from "./HourChart";
import type { HourPlot } from "./HourChart";
import { FIGURE_NAME, LIST_KEYS, TIME_FIGURES, commitOf, inBuilds, lapse } from "./usage-builds";
import type { Build, UsageBuilds } from "./usage-builds";
import { SHOWN, latestHour } from "./usage-hours";
import type { UsageHours } from "./usage-hours";
import { Band, Bar, Fig, Scope } from "./usage-parts";
import { pct, shareOf } from "./usage-view";
import {
  BUCKETS,
  BUCKET_LABEL,
  BUCKET_MEANS,
  bySpent,
  farOver,
  span,
  spanSaid,
  sumOf,
} from "./usage-time";
import type { Bucket, TimeJob, TimeSplit, UsageTime } from "./usage-time";

/**
 * Where the time went, laid out as the tokens view is: the slowest Jobs on the left, each with its split as one thin bar,
 * and in the centre the four figures hour by hour over the split of the whole window. Picking a Job puts its own split in the
 * window's place, each bucket against the usual of that Job's workflow where it is far over.
 *
 * Keys: `↑↓ j k` move down the Jobs, `↵ o` opens the one under the cursor, `← → h l` step the hours and `Esc` clears the
 * hour, then the Job, then the build. Picking a build, in the Builds panel or on a marker, narrows the Jobs to the ones that
 * ran on it and the split to its own, and shades its span on the hour chart.
 */

/** A bucket's name over its swatch, with what the bucket means on hover. */
function BucketName({ bucket }: { bucket: Bucket }) {
  return (
    <Tooltip label={BUCKET_MEANS[bucket]}>
      <span className="armada-time__bucket" data-time={bucket}>
        <span className="armada-time__swatch" aria-hidden />
        <span className="armada-usage__name">{BUCKET_LABEL[bucket]}</span>
      </span>
    </Tooltip>
  );
}

/** A Job's split as one thin bar: the buckets in their own order, the whole column wide so the shapes read against each other. */
function SplitBar({ split }: { split: TimeSplit }) {
  const total = sumOf(split);
  return (
    <span className="armada-time__track" aria-hidden>
      <span className="armada-time__stack">
        {BUCKETS.filter((one) => split[one] > 0).map((one) => (
          <Tooltip asChild key={one} label={`${BUCKET_LABEL[one]} ${span(split[one])}, ${Math.round((split[one] / total) * 100)}%`}>
            <span
              className="armada-time__part"
              data-time={one}
              style={{
                flexGrow: split[one],
              }}
            />
          </Tooltip>
        ))}
      </span>
    </span>
  );
}

function JobRow({
  one,
  cursor,
  picked,
  onPick,
}: {
  one: TimeJob;
  cursor: boolean;
  picked: boolean;
  onPick: () => void;
}) {
  const total = sumOf(one.split);
  const longest = bySpent(one.split)[0];
  return (
    <li
      role="option"
      id={`time-${one.id}`}
      aria-selected={picked}
      aria-label={`${one.title}, ${spanSaid(total)}${longest === undefined ? "" : `, most in ${BUCKET_LABEL[longest]}`}`}
      data-cursor={cursor || undefined}
      className="armada-time__job"
      onClick={onPick}
    >
      <Tooltip label={one.title}>
        <span className="armada-usage__title">{one.title}</span>
      </Tooltip>
      <SplitBar split={one.split} />
      <Fig tip="Total time, every bucket">{span(total)}</Fig>
    </li>
  );
}

/**
 * The buckets of one split, most time first, each against the whole. Beside the time sits the usual for the Job's workflow
 * where the bucket is far over it, and the share of the whole where no usual is given.
 */
export function SplitRows({
  split,
  usual,
  workflow,
}: {
  split: TimeSplit;
  /** The usual Job of the workflow, to read each bucket against. Absent reads each as a share. */
  usual?: TimeSplit | undefined;
  workflow?: string | undefined;
}) {
  const total = sumOf(split);
  const against = workflow !== undefined;
  return (
    <>
      <div className="armada-time__heads" aria-hidden>
        <span />
        <span />
        <span>Time</span>
        <span>{against ? "Usual" : "Share"}</span>
      </div>
      <ul className="armada-time__rows" aria-label={against ? "Job split" : "Split"}>
        {bySpent(split).map((one) => {
          const over = against && farOver(split[one], usual?.[one]);
          return (
            <li
              key={one}
              className="armada-time__row"
              data-over={over || undefined}
              aria-label={`${BUCKET_LABEL[one]}, ${spanSaid(split[one])}${over ? `, usual ${spanSaid(usual![one])}` : ""}`}
            >
              <BucketName bucket={one} />
              <span className="armada-usage__track" aria-hidden>
                <span
                  className="armada-time__fill"
                  data-time={one}
                  style={{
                    width: `${shareOf(split[one], total)}%`,
                  }}
                />
                {over ? (
                  <span
                    className="armada-time__usual"
                    style={{
                      left: `${shareOf(usual![one], total)}%`,
                    }}
                  />
                ) : null}
              </span>
              <Fig over={over} tip={`${BUCKET_LABEL[one]}, ${spanSaid(split[one])}`}>
                {span(split[one])}
              </Fig>
              {against ? (
                over ? (
                  <Fig tip={`Usual for ${workflow}: the median Job, ${spanSaid(usual![one])}`}>
                    {span(usual![one])}
                  </Fig>
                ) : (
                  <span />
                )
              ) : (
                <Fig tip="Share of all time">{pct(shareOf(split[one], total))}</Fig>
              )}
            </li>
          );
        })}
      </ul>
    </>
  );
}

/** One Job's split, for the panel that holds it: its title and total over its buckets, each against its workflow's usual. */
export function JobSplit({
  job,
  usual,
  flush,
}: {
  job: TimeJob;
  usual: TimeSplit | undefined;
  /** Drawn in a Job's own tab rather than a panel: no side padding of its own. */
  flush?: boolean;
}) {
  return (
    <div className="armada-time__jobsplit" data-flush={flush || undefined}>
      <div className="armada-usage__scope">
        <Tooltip label={job.title}>
          <span className="armada-usage__title">{job.title}</span>
        </Tooltip>
        <Fig tip={`Total time, ${job.workflow}`}>{span(sumOf(job.split))}</Fig>
      </div>
      <SplitRows split={job.split} usual={usual} workflow={job.workflow} />
    </div>
  );
}

const JOB_BARS = [
  "100%",
  "86%",
  "74%",
  "62%",
  "50%",
  "40%",
];

/** The loaded layout's shapes in skeleton bars, so the figures land on them. */
function PendingJobs() {
  return (
    <ul className="armada-time__jobs" aria-hidden>
      {JOB_BARS.map((track, at) => (
        <li key={track} className="armada-time__job">
          <Bar width={`${50 + ((at * 17) % 30)}%`} />
          <span className="armada-time__track">
            <Skeleton className="armada-usage__skel-track" width={track} />
          </span>
          <Bar width="80%" end />
        </li>
      ))}
    </ul>
  );
}

function PendingRows() {
  return (
    <ul className="armada-time__rows" aria-hidden>
      {BUCKETS.slice(0, 6).map((one, at) => (
        <li key={one} className="armada-time__row">
          <Bar width="70%" />
          <Skeleton className="armada-usage__skel-track" width={`${100 - at * 14}%`} />
          <Bar width="80%" end />
          <Bar width="80%" end />
        </li>
      ))}
    </ul>
  );
}

const PLOT_HEIGHT = 96;
const UNIT: Readonly<Record<(typeof TIME_FIGURES)[number], { unit: string; per: number }>> = {
  model: {
    unit: "s",
    per: 1,
  },
  tools: {
    unit: "s",
    per: 1,
  },
  checks: {
    unit: "min",
    per: 60,
  },
  pull_request: {
    unit: "min",
    per: 60,
  },
};

/** The four figures as small charts, each with its own scale in seconds or minutes. */
const plotsOf = (hours: UsageHours): HourPlot[] =>
  TIME_FIGURES.map((figure) => ({
    key: figure,
    figure,
    title: FIGURE_NAME[figure],
    unit: UNIT[figure].unit,
    height: PLOT_HEIGHT,
    say: (value) => lapse(value * UNIT[figure].per),
    series: [
      {
        key: figure,
        name: FIGURE_NAME[figure],
        time: figure,
        values: hours.map((one) => {
          const seconds = one[figure];
          return seconds === undefined ? undefined : seconds / UNIT[figure].per;
        }),
      },
    ],
  }));

const PENDING_PLOTS = TIME_FIGURES.map((figure) => ({
  key: figure,
  title: figure,
  height: PLOT_HEIGHT,
}));

const HOURS_LABEL =
  "Median per hour, last 48 hours: model wait per turn and tools per call in seconds, Checks per run and pull request wait in minutes";

type TimeViewProps = {
  /** Fleet's reading of where the time went. `pending` until it has answered: the panels draw the shape of what is coming. */
  time: UsageTime | "pending";
  /** The builds on offer. Absent draws no Builds panel. */
  builds?: UsageBuilds | "pending" | undefined;
  /** The last 48 hours, an hour a point. `pending` draws the chart's shape; absent draws no chart. */
  hours?: UsageHours | "pending" | undefined;
  /** The picked build: the panels narrow to it. */
  build?: Build | undefined;
  onBuild?: (id: string | undefined) => void;
};

export function TimeView({ time, builds, hours, build, onBuild }: TimeViewProps) {
  const [cursor, setCursor] = useState(-1);
  const [pick, setPick] = useState(-1);
  const [hour, setHour] = useState<number | undefined>(undefined);
  const [hover, setHover] = useState<number | undefined>(undefined);
  const ready = time === "pending" ? undefined : time;
  const jobs = (build === undefined ? ready?.jobs : ready?.jobs.filter((one) => build.jobs.includes(one.id))) ?? [];
  const drawn = hours === undefined || hours === "pending" ? undefined : hours;
  const listed = builds === undefined || builds === "pending" ? undefined : builds;
  const picked = jobs[pick];
  const latest = useRef({
    cursor,
    pick,
    hour,
    count: jobs.length,
    last: drawn === undefined ? 0 : latestHour(drawn),
    build,
  });
  latest.current = {
    cursor,
    pick,
    hour,
    count: jobs.length,
    last: drawn === undefined ? 0 : latestHour(drawn),
    build,
  };

  useEffect(() => {
    function pressed(event: KeyboardEvent): void {
      if (event.metaKey || event.ctrlKey || event.altKey) return;
      if (document.querySelector('[role="dialog"], [role="alertdialog"], [role="menu"]') !== null)
        return;
      const target = event.target;
      if (
        target instanceof HTMLElement &&
        (target.isContentEditable ||
          [
            "INPUT",
            "TEXTAREA",
            "SELECT",
          ].includes(target.tagName))
      )
        return;
      const onControl =
        target instanceof HTMLElement && target.closest("button, [role=tab]") !== null;
      const onTab = target instanceof HTMLElement && target.closest("[role=tab]") !== null;
      if (inBuilds(target) && LIST_KEYS.includes(event.key)) return;
      const now = latest.current;
      const stepHour = (by: number) =>
        setHour((at) => (at === undefined ? now.last : Math.min(SHOWN - 1, Math.max(0, at + by))));
      if (event.key === "ArrowDown" || event.key === "j")
        setCursor((at) => Math.min(now.count - 1, at + 1));
      else if (event.key === "ArrowUp" || event.key === "k") setCursor((at) => Math.max(0, at - 1));
      else if ((event.key === "Enter" || event.key === "o") && !onControl) {
        if (now.cursor < 0) return;
        setPick(now.cursor);
      } else if (event.key === "Escape" && now.hour !== undefined) setHour(undefined);
      else if (event.key === "Escape" && now.pick >= 0) setPick(-1);
      else if (event.key === "Escape" && now.build !== undefined) onBuild?.(undefined);
      else if ((event.key === "ArrowRight" || event.key === "l") && !onTab) stepHour(1);
      else if ((event.key === "ArrowLeft" || event.key === "h") && !onTab) stepHour(-1);
      else return;
      event.preventDefault();
    }
    window.addEventListener("keydown", pressed);
    return () => window.removeEventListener("keydown", pressed);
  }, []);

  useEffect(() => {
    setCursor(-1);
    setPick(-1);
    setHour(undefined);
  }, [
    build,
  ]);

  useEffect(() => {
    const row = jobs[cursor];
    if (row !== undefined)
      document.getElementById(`time-${row.id}`)?.scrollIntoView?.({
        block: "nearest",
      });
  }, [
    cursor,
    jobs,
  ]);

  const active = jobs[cursor];
  const shown = hover ?? hour;

  return (
    <div className="armada-time" aria-busy={ready === undefined || undefined}>
      <div className="armada-time__left">
        <section className="armada-usage__panel" aria-label="Slowest Jobs">
          <Band
            keys={[
              "↑↓",
              "↵",
              "Esc",
            ]}
          >
            Slowest Jobs
          </Band>
          {ready === undefined ? (
            <PendingJobs />
          ) : (
            <>
              <div className="armada-time__jobheads" aria-hidden>
                <span />
                <span>Split</span>
                <span>Total</span>
              </div>
              <ul
                className="armada-time__jobs"
                role="listbox"
                aria-label="Slowest Jobs"
                aria-activedescendant={active === undefined ? undefined : `time-${active.id}`}
                tabIndex={0}
              >
                {jobs.map((one, at) => (
                  <JobRow
                    key={one.id}
                    one={one}
                    cursor={at === cursor}
                    picked={at === pick}
                    onPick={() => {
                      setCursor(at);
                      setPick(at);
                    }}
                  />
                ))}
              </ul>
            </>
          )}
        </section>
        {builds === undefined ? null : (
          <BuildsPanel builds={builds} view="time" pick={build?.id} onPick={(id) => onBuild?.(id)} />
        )}
      </div>

      <div className="armada-time__centre">
        <section className="armada-usage__panel" aria-label="Hour by hour">
          <Band
            keys={[
              "←→",
            ]}
          >
            Hour by hour
          </Band>
          {ready === undefined || hours === "pending" ? (
            <>
              <div className="armada-usage__scope" aria-hidden>
                <Bar width="8rem" />
              </div>
              <PendingHours plots={PENDING_PLOTS} />
            </>
          ) : drawn === undefined ? null : (
            <>
              <Scope title="Median per hour" />
              <HourChart
                hours={drawn}
                plots={plotsOf(drawn)}
                label={HOURS_LABEL}
                builds={listed ?? []}
                pick={build?.id}
                onPick={(id) => onBuild?.(id)}
                hour={shown}
                onHover={setHover}
                onSelect={setHour}
              />
              <HourRows hours={drawn} plots={plotsOf(drawn)} label="Median per hour" />
            </>
          )}
        </section>

        <section className="armada-usage__panel" aria-label="Split">
          <Band>The split</Band>
          {ready === undefined ? (
            <>
              <div className="armada-usage__scope" aria-hidden>
                <Bar width="8rem" />
              </div>
              <PendingRows />
            </>
          ) : (
            <>
              <Scope title={build === undefined ? "All Jobs" : `Build ${commitOf(build)}`} />
              <SplitRows split={build === undefined ? ready.total : build.split} />
            </>
          )}
        </section>
      </div>
      <Sheet
        open={picked !== undefined}
        title={picked?.title ?? "Job split"}
        subtitle={picked === undefined ? undefined : span(sumOf(picked.split))}
        kind="job-split"
        onClose={() => setPick(-1)}
      >
        {picked === undefined || ready === undefined ? null : (
          <SplitRows split={picked.split} usual={ready.usual[picked.workflow]} workflow={picked.workflow} />
        )}
      </Sheet>
    </div>
  );
}
