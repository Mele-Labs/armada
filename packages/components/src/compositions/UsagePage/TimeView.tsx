import { useEffect, useRef, useState } from "react";
import type { MouseEvent } from "react";

import { Skeleton } from "../../primitives/Skeleton/Skeleton";
import { Tooltip } from "../../primitives/Tooltip/Tooltip";
import { BuildsPanel } from "./UsageBuilds";
import { LIST_KEYS, inBuilds } from "./usage-builds";
import type { Build, UsageBuilds } from "./usage-builds";
import { Band, Bar, Fig, Scope } from "./usage-parts";
import { pct, shareOf } from "./usage-view";
import {
  ACTIVE,
  BUCKETS,
  BUCKET_LABEL,
  BUCKET_MEANS,
  bySpent,
  dayName,
  farOver,
  span,
  spanSaid,
  sumOf,
} from "./usage-time";
import type { Bucket, TimeJob, TimeSplit, TimeWeek, UsageTime } from "./usage-time";

/**
 * Where the time went, laid out as the tokens view is: the slowest Jobs on the left, each with its split as one thin bar,
 * and in the centre the median Job week by week over the split of the whole window. Picking a Job puts its own split in the
 * window's place, each bucket against the usual of that Job's workflow where it is far over.
 *
 * Keys: `↑↓ j k` move down the Jobs, `↵ o` opens the one under the cursor, `← → h l` step the weeks and `Esc` clears the
 * week, then the Job, then the build. Picking a build in the Builds panel narrows the Jobs to the ones that ran on it, the weeks
 * to the one it began in and the split to its own.
 */

const NONE: TimeSplit = Object.fromEntries(BUCKETS.map((one) => [one, 0])) as unknown as TimeSplit;

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

function Legend() {
  return (
    <ul className="armada-usage__legend" aria-label="Buckets">
      {BUCKETS.map((one) => (
        <li key={one}>
          <BucketName bucket={one} />
        </li>
      ))}
    </ul>
  );
}

const W = 520;
const H = 226;
const LEFT = 36;
const RIGHT = 96;
const TOP = 16;
const BASE = 196;
const TICK = 30 * 60;

type Part = {
  bucket: Bucket;
  low: number;
  high: number;
};

/** The active buckets stacked bottom to top in their own order, so a bucket keeps its place week to week. */
function stack(split: TimeSplit): Part[] {
  let low = 0;
  return ACTIVE.flatMap((bucket) => {
    if (split[bucket] <= 0) return [];
    const part = {
      bucket,
      low,
      high: low + split[bucket],
    };
    low += split[bucket];
    return [
      part,
    ];
  });
}

function WeekChart({
  weeks,
  week,
  onWeek,
}: {
  weeks: readonly TimeWeek[];
  week: number | undefined;
  onWeek: (week: number | undefined) => void;
}) {
  const slot = (W - LEFT - RIGHT) / weeks.length;
  const bar = slot - 18;
  const barX = (at: number) => LEFT + at * slot + (slot - bar) / 2;
  const totals = weeks.map((one) => sumOf(one.median, ACTIVE));
  const peak = Math.max(...totals);
  const top = Math.max(TICK, Math.ceil(peak / TICK) * TICK);
  const y = (seconds: number) => BASE - ((BASE - TOP) * seconds) / top;
  const ticks = Array.from(
    {
      length: Math.round(top / TICK) + 1,
    },
    (_, i) => i * TICK,
  );
  const column = (x: number, low: number, high: number, round: boolean) => {
    const r = round ? Math.min(4, (y(low) - y(high)) / 2) : 0;
    return `M${x},${y(low)} V${y(high) + r} Q${x},${y(high)} ${x + r},${y(high)} H${x + bar - r} Q${x + bar},${y(high)} ${x + bar},${y(high) + r} V${y(low)} Z`;
  };
  const drawn = (parts: Part[]) => parts.filter((one) => y(one.low) - y(one.high) >= 1);
  const place = (event: MouseEvent<HTMLDivElement>) => {
    const box = event.currentTarget.getBoundingClientRect();
    const x = ((event.clientX - box.left) / box.width) * W;
    const at = Math.floor((x - LEFT) / slot);
    onWeek(at >= 0 && at < weeks.length ? at : undefined);
  };
  const drawnAt = totals.flatMap((total, at) => (total > 0 ? [at] : []));
  const first = drawnAt[0];
  const last = drawnAt[drawnAt.length - 1];
  const shown = week === undefined ? undefined : weeks[week];
  return (
    <div className="armada-usage__plot" onMouseMove={place} onMouseLeave={() => onWeek(undefined)}>
      <svg
        className="armada-usage__chart"
        viewBox={`0 0 ${W} ${H}`}
        role="img"
        aria-label="Median active time per Job, by week, in minutes, by bucket"
      >
        {ticks.map((tick) => (
          <g key={tick}>
            <line
              className="armada-usage__grid"
              x1={LEFT}
              x2={W - RIGHT}
              y1={y(tick)}
              y2={y(tick)}
            />
            <text className="armada-usage__tick" x={LEFT - 6} y={y(tick) + 4} textAnchor="end">
              {tick / 60}
            </text>
          </g>
        ))}
        {week === undefined ? null : (
          <rect
            className="armada-usage__daywash"
            x={LEFT + week * slot}
            y={TOP - 8}
            width={slot}
            height={BASE - TOP + 8}
          />
        )}
        {weeks.map((one, at) => {
          const parts = drawn(stack(one.median));
          return (
            <g key={one.from}>
              {parts.map((part, i) => (
                <path
                  key={part.bucket}
                  className="armada-time__seg"
                  data-time={part.bucket}
                  d={column(barX(at), part.low, part.high, i === parts.length - 1)}
                />
              ))}
            </g>
          );
        })}
        {(last === undefined ? [] : drawn(stack(weeks[last]!.median))).map((part) =>
          y(part.low) - y(part.high) < 14 ? null : (
            <text
              key={part.bucket}
              className="armada-usage__direct"
              x={barX(last!) + bar + 6}
              y={(y(part.low) + y(part.high)) / 2 + 4}
            >
              {BUCKET_LABEL[part.bucket]}
            </text>
          ),
        )}
        {(first === undefined || last === undefined
          ? []
          : [
              ...new Set([
                first,
                last,
              ]),
            ]
        ).map((at) => (
          <text
            key={at}
            className="armada-usage__peak"
            x={barX(at) + bar / 2}
            y={y(totals[at]!) - 6}
            textAnchor="middle"
          >
            {span(totals[at]!)}
          </text>
        ))}
        {weeks.map((one, at) => (
          <text
            key={one.from}
            className="armada-usage__tick"
            data-on={week === at || undefined}
            x={barX(at) + bar / 2}
            y={BASE + 16}
            textAnchor="middle"
          >
            {dayName(one.from)}
          </text>
        ))}
        <line className="armada-usage__base" x1={LEFT} x2={W - RIGHT} y1={BASE} y2={BASE} />
        {week === undefined ? null : (
          <line
            className="armada-usage__cross"
            x1={barX(week) + bar / 2}
            x2={barX(week) + bar / 2}
            y1={TOP - 8}
            y2={BASE}
          />
        )}
      </svg>
      <div className="armada-usage__hits" aria-hidden>
        {weeks.map((one, at) => (
          <button
            key={one.from}
            type="button"
            tabIndex={-1}
            className="armada-usage__hit"
            style={{
              left: `${((LEFT + at * slot) / W) * 100}%`,
              width: `${(slot / W) * 100}%`,
            }}
            onClick={() => onWeek(at)}
          />
        ))}
      </div>
      {shown === undefined || week === undefined ? null : (
        <div
          className="armada-usage__pop"
          role="status"
          data-flip={week >= weeks.length / 2 || undefined}
          style={{
            left: `${((barX(week) + bar / 2) / W) * 100}%`,
          }}
        >
          <span className="armada-usage__popday">{`Week of ${dayName(shown.from)}`}</span>
          {stack(shown.median)
            .reverse()
            .map((part) => (
              <span key={part.bucket} className="armada-usage__popline">
                <span className="armada-time__swatch" data-time={part.bucket} aria-hidden />
                <span>{BUCKET_LABEL[part.bucket]}</span>
                <span className="armada-usage__fig">{span(shown.median[part.bucket])}</span>
              </span>
            ))}
          <span className="armada-usage__popline" data-total>
            <span />
            <span>Active</span>
            <span className="armada-usage__fig">{span(totals[week]!)}</span>
          </span>
        </div>
      )}
    </div>
  );
}

/** A Job's split as one thin bar: the buckets in their own order, the whole column wide so the shapes read against each other. */
function SplitBar({ split }: { split: TimeSplit }) {
  return (
    <span className="armada-time__track" aria-hidden>
      <span className="armada-time__stack">
        {BUCKETS.filter((one) => split[one] > 0).map((one) => (
          <span
            key={one}
            className="armada-time__part"
            data-time={one}
            style={{
              flexGrow: split[one],
            }}
          />
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

const WEEK_FILL = [
  0.5,
  0.54,
  0.6,
  0.7,
  0.82,
  0.95,
];

/** Six bars on a base line, laid over the same box the chart's own drawing fills. */
function PendingChart() {
  const slot = (W - LEFT - RIGHT) / WEEK_FILL.length;
  const bar = slot - 18;
  return (
    <div className="armada-usage__plot" aria-hidden>
      <div
        className="armada-usage__skel-chart"
        style={{
          aspectRatio: `${W} / ${H}`,
        }}
      >
        {WEEK_FILL.map((fill, at) => (
          <Skeleton
            key={at}
            className="armada-usage__skel-column"
            style={{
              left: `${((LEFT + at * slot + (slot - bar) / 2) / W) * 100}%`,
              width: `${(bar / W) * 100}%`,
              height: `${(((BASE - TOP) * fill) / H) * 100}%`,
              bottom: `${((H - BASE) / H) * 100}%`,
            }}
          />
        ))}
      </div>
    </div>
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

function PendingLegend() {
  return (
    <ul className="armada-usage__legend" aria-hidden>
      {BUCKETS.map((one) => (
        <li key={one}>
          <Bar width="100%" />
        </li>
      ))}
    </ul>
  );
}

type TimeViewProps = {
  /** Fleet's reading of where the time went. `pending` until it has answered: the panels draw the shape of what is coming. */
  time: UsageTime | "pending";
  /** The builds on offer. Absent draws no Builds panel. */
  builds?: UsageBuilds | "pending" | undefined;
  /** The picked build: the panels narrow to it. */
  build?: Build | undefined;
  onBuild?: (id: string | undefined) => void;
};

export function TimeView({ time, builds, build, onBuild }: TimeViewProps) {
  const [cursor, setCursor] = useState(-1);
  const [pick, setPick] = useState(-1);
  const [week, setWeek] = useState<number | undefined>(undefined);
  const [hover, setHover] = useState<number | undefined>(undefined);
  const ready = time === "pending" ? undefined : time;
  const jobs = (build === undefined ? ready?.jobs : ready?.jobs.filter((one) => build.jobs.includes(one.id))) ?? [];
  const weeks =
    ready === undefined || build === undefined
      ? ready?.weeks
      : ready.weeks.map((one, at) => ({
          from: one.from,
          median: at === build.week ? build.median : NONE,
        }));
  const picked = jobs[pick];
  const latest = useRef({
    cursor,
    pick,
    week,
    count: jobs.length,
    weeks: ready?.weeks.length ?? 0,
    build,
  });
  latest.current = {
    cursor,
    pick,
    week,
    count: jobs.length,
    weeks: ready?.weeks.length ?? 0,
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
      const stepWeek = (by: number) =>
        setWeek((at) =>
          at === undefined
            ? by > 0
              ? 0
              : now.weeks - 1
            : Math.min(now.weeks - 1, Math.max(0, at + by)),
        );
      if (event.key === "ArrowDown" || event.key === "j")
        setCursor((at) => Math.min(now.count - 1, at + 1));
      else if (event.key === "ArrowUp" || event.key === "k") setCursor((at) => Math.max(0, at - 1));
      else if ((event.key === "Enter" || event.key === "o") && !onControl) {
        if (now.cursor < 0) return;
        setPick(now.cursor);
      } else if (event.key === "Escape" && now.week !== undefined) setWeek(undefined);
      else if (event.key === "Escape" && now.pick >= 0) setPick(-1);
      else if (event.key === "Escape" && now.build !== undefined) onBuild?.(undefined);
      else if ((event.key === "ArrowRight" || event.key === "l") && !onTab) stepWeek(1);
      else if ((event.key === "ArrowLeft" || event.key === "h") && !onTab) stepWeek(-1);
      else return;
      event.preventDefault();
    }
    window.addEventListener("keydown", pressed);
    return () => window.removeEventListener("keydown", pressed);
  }, []);

  useEffect(() => {
    setCursor(-1);
    setPick(-1);
    setWeek(undefined);
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
  const shown = hover ?? week;

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
        <section className="armada-usage__panel" aria-label="Week by week">
          <Band
            keys={[
              "←→",
            ]}
          >
            Week by week
          </Band>
          {ready === undefined ? (
            <>
              <div className="armada-usage__scope" aria-hidden>
                <Bar width="8rem" />
              </div>
              <PendingChart />
              <PendingLegend />
            </>
          ) : (
            <>
              <Scope title="Median Job, active time" unit="min" />
              <WeekChart weeks={weeks!} week={shown} onWeek={setHover} />
              <Legend />
            </>
          )}
        </section>

        <section className="armada-usage__panel" aria-label={picked === undefined ? "Split" : "Job split"}>
          <Band>{picked === undefined ? "The split" : "Job split"}</Band>
          {ready === undefined ? (
            <>
              <div className="armada-usage__scope" aria-hidden>
                <Bar width="8rem" />
              </div>
              <PendingRows />
            </>
          ) : picked === undefined ? (
            <>
              <Scope title={build === undefined ? "All Jobs" : `Build ${build.id}`} />
              <SplitRows split={build === undefined ? ready.total : build.split} />
            </>
          ) : (
            <JobSplit job={picked} usual={ready.usual[picked.workflow]} />
          )}
        </section>
      </div>
    </div>
  );
}
