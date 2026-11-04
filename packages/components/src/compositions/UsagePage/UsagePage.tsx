import { ArrowDown, ArrowUp } from "lucide-react";
import { useEffect, useMemo, useRef, useState } from "react";
import type {
  SessionUsageSummary,
  UsageBreakdown,
  UsageRules as UsageRulesAnswer,
  UsageSummary,
} from "@armada/protocol";

import { Kbd } from "../../primitives/Kbd/Kbd";
import { Skeleton } from "../../primitives/Skeleton/Skeleton";
import { Tabs } from "../../primitives/Tabs/Tabs";
import { Tooltip } from "../../primitives/Tooltip/Tooltip";
import {
  BOUNDS,
  HOURS,
  M,
  WEEKDAYS,
  bashLabel,
  blockLabel,
  chartDays,
  dayLabel,
  foldModels,
  heatOf,
  peakCell,
  pct,
  percentDelta,
  pointsDelta,
  shareOf,
  shareRows,
  rowsOf,
  ruleViews,
  stepOf,
  tokenRows,
  toolLabel,
} from "./usage-view";
import type { ChartDay, Row, ShareRow } from "./usage-view";
import { UsageRules } from "./UsageRules";
import { Band, Bar, Fig, Scope } from "./usage-parts";
import type { UsageTime } from "./usage-time";
import { TimeView } from "./TimeView";
import { SpentView } from "./SpentView";
import type { UsageSpent } from "./usage-spent";
import { BuildsPanel } from "./UsageBuilds";
import { HourChart, HourRows, PendingHours } from "./HourChart";
import type { HourPlot } from "./HourChart";
import { LIST_KEYS, commitOf, inBuilds } from "./usage-builds";
import type { UsageBuilds } from "./usage-builds";
import { SHOWN, blockOfHour, latestHour, weekdayOfHour } from "./usage-hours";
import type { UsageHours } from "./usage-hours";

/**
 * Where the tokens went, laid out as the cockpit is: the Jobs that burned it on the left with one breakdown under them,
 * the burn per hour over the last 48 hours and the day by hour map in the centre, the headline figures on the
 * right. It draws Fleet's usage summary, and what the summary lacks it leaves empty.
 *
 * Keys: `↓ j` and `↑ k` move down the Jobs, `↵ o` picks the one under the cursor, `Esc` clears the hour and then
 * the pick, `m` `t` `b` `y` `d` set the breakdown to model, tool, Bash use, token type or day, `← → h l` step the
 * hour on the burn chart, which the map's cell follows, and `[ ]` step the 3 hour block on the map (the first of these lands
 * on the busiest cell). `1` to `9` pick the
 * usage rule at that place, and `Esc` clears it before the Job pick. Where Time is offered, `v` switches the page between
 * tokens and time, and the Time view keeps its own keys. Where Spent on is offered too, `v` steps through the three. Where builds are offered, `c` moves focus to the Builds list, whose
 * keys are its own while it holds focus (`↑↓ j k` move, `↵ o` pick), and `Esc` clears the build pick after the Job pick. A picked
 * build narrows the Jobs, the day and hour map and the tiles on this view, and the Jobs and the split on Time; on both views
 * its span is shaded on the hour chart, and its marker picks it.
 */

export type UsagePageProps = {
  /** Fleet's summary of the last seven days. Absent until it has answered: the panels draw the shape of what is coming. */
  summary?: UsageSummary | undefined;
  /** The titles Bridge holds for Jobs, by id. A Job with none shows its short id. */
  titles?: ReadonlyMap<string, string> | undefined;
  /** The 7 day plan window's percent used, from the plan's own reading. Absent draws no plan tile. */
  planPercent?: number | undefined;
  /** Fleet's usage rules with their trips. Absent draws no rules panel. */
  rules?: UsageRulesAnswer | undefined;
  /** Where the time went. Absent offers no Time view; `pending` offers it with its panels drawing the shape of what is coming. */
  time?: UsageTime | "pending" | undefined;
  /** The builds the figures are read against. Absent offers no Builds panel; `pending` draws its shape. */
  builds?: UsageBuilds | "pending" | undefined;
  /** The last 48 hours, an hour a point. Absent draws no hour chart; `pending` draws its shape. */
  hours?: UsageHours | "pending" | undefined;
  /** What the tokens were spent on, by window. Absent offers no Spent on view. */
  spent?: UsageSpent | undefined;
  /** Told the id of the build picked, and again with nothing when it is cleared: a build's tokens are read when it is picked. */
  onBuild?: ((id: string | undefined) => void) | undefined;
};

type View = "tokens" | "spent" | "time";
const VIEWS: {
  id: View;
  label: string;
}[] = [
  {
    id: "tokens",
    label: "Tokens",
  },
  {
    id: "spent",
    label: "Spent on",
  },
  {
    id: "time",
    label: "Time",
  },
];

type Cut = "model" | "tool" | "bash" | "token" | "day";
const CUTS = [
  {
    id: "model",
    label: "By model",
  },
  {
    id: "tool",
    label: "By tool",
  },
  {
    id: "bash",
    label: "By Bash use",
  },
  {
    id: "token",
    label: "By token type",
  },
  {
    id: "day",
    label: "By day",
  },
];
const CUT_KEY: Record<string, Cut> = {
  m: "model",
  t: "tool",
  b: "bash",
  y: "token",
  d: "day",
};

type Cell = {
  d: number;
  b: number;
};
type Source = {
  title: string;
  usage: UsageBreakdown;
  days: ChartDay[];
  session?: SessionUsageSummary;
  /** A Job or a build is picked: the figures are its share of the week. */
  scoped: boolean;
};

type Delta = {
  up: boolean;
  text: string;
  tip: string;
};

/** The plan window's share where its tile lights, and the context past which a Job's peak does. */
const PLAN_NEAR = 80;
const PLAN_OVER = 95;
const PEAK_OVER = 400_000;

/** One headline figure, a delta against the period before, and a thin meter where the figure is a share of a limit. */
function Tile({
  name,
  figure,
  tip,
  meter,
  delta,
}: {
  name: string;
  figure: string;
  tip: string;
  meter?: number;
  delta?: Delta | undefined;
}) {
  return (
    <li
      className="armada-usage__tile"
      aria-label={name}
      // Every figure here is burn, so one going up is going the wrong way.
      data-way={delta === undefined ? undefined : delta.up ? "worse" : "better"}
      data-level={
        meter === undefined
          ? undefined
          : meter >= PLAN_OVER
            ? "over"
            : meter >= PLAN_NEAR
              ? "near"
              : undefined
      }
    >
      <span className="armada-usage__name">{name}</span>
      <span className="armada-usage__big">
        <Fig tip={tip}>{figure}</Fig>
      </span>
      {meter === undefined ? null : (
        <span
          className="armada-usage__meter"
          role="meter"
          aria-label={name}
          aria-valuemin={0}
          aria-valuemax={100}
          aria-valuenow={meter}
        >
          <span
            className="armada-usage__fill"
            style={{
              width: `${meter}%`,
            }}
          />
        </span>
      )}
      <span className="armada-usage__deltaslot">
        {delta === undefined ? null : (
          <Tooltip label={delta.tip}>
            <span
              className="armada-usage__delta"
              data-way={delta.up ? "worse" : "better"}
              aria-label={delta.tip}
            >
              {delta.up ? <ArrowUp size={12} aria-hidden /> : <ArrowDown size={12} aria-hidden />}
              {delta.text}
            </span>
          </Tooltip>
        )}
      </span>
    </li>
  );
}

function points(now: number, before: number, hadBefore: boolean): Delta | undefined {
  const moved = pointsDelta(now, before, hadBefore);
  return moved === undefined
    ? undefined
    : {
        up: moved.up,
        text: `${moved.points} pts`,
        tip: `${moved.up ? "Up" : "Down"} ${moved.points} points on the week before`,
      };
}

function Tiles({
  summary,
  source,
  planPercent,
}: {
  summary: UsageSummary;
  source: Source;
  planPercent: number | undefined;
}) {
  const { usage, scoped } = source;
  const today = source.days[6]!.units;
  const change = percentDelta(today, source.days[5]!.units);
  const planShare =
    planPercent === undefined
      ? undefined
      : !scoped
        ? planPercent
        : Math.round(shareOf(usage.units, summary.total.units) * planPercent) / 100;
  const hadBefore = summary.prior.units > 0;
  const subagent = shareOf(usage.subagent_units, usage.units);
  const above = shareOf(usage.above_200k_units, usage.units);
  return (
    <ul className="armada-usage__tiles" aria-label="Headline figures">
      {planShare === undefined ? null : (
        <Tile
          name="Weekly plan used"
          figure={`${Math.round(planShare * 10) / 10}%`}
          tip={
            !scoped
              ? "Share of the 7 day plan window used"
              : source.session === undefined
                ? "Share of the 7 day plan window this build used"
                : "Share of the 7 day plan window this job used"
          }
          meter={planShare}
        />
      )}
      <Tile
        name="Today's burn"
        figure={`${M(today)}M`}
        tip={`Cost units, millions, ${dayLabel(source.days[6]!.day)}`}
        delta={
          change === undefined
            ? undefined
            : {
                up: change.up,
                text: `${change.percent.toFixed(1)}%`,
                tip: `${change.up ? "Up" : "Down"} ${change.percent.toFixed(1)} percent on yesterday`,
              }
        }
      />
      <Tile
        name="Burn in subagents"
        figure={`${Math.round(subagent)}%`}
        tip="Share of the burn that ran in subagents"
        delta={
          !scoped ? points(subagent, summary.prior.subagent_share * 100, hadBefore) : undefined
        }
      />
      <Tile
        name="Above 200k context"
        figure={`${Math.round(above)}%`}
        tip="Share of the burn from requests above 200k tokens of context"
        delta={!scoped ? points(above, summary.prior.above_200k_share * 100, hadBefore) : undefined}
      />
    </ul>
  );
}

/** A share of the burn per name, one bar a row, all to one scale. */
function ShareRows({
  label,
  rows,
  tip = "Share of the burn",
}: {
  label: string;
  rows: ShareRow[];
  tip?: string;
}) {
  const top = Math.max(0, ...rows.map((one) => one.share));
  return (
    <ul className="armada-usage__shares" aria-label={label}>
      {rows.map((one) => (
        <li key={one.name} aria-label={`${one.name}, ${pct(one.share)} of the burn`}>
          <Tooltip label={one.name}>
            <span className="armada-usage__name">{one.name}</span>
          </Tooltip>
          <span className="armada-usage__track" aria-hidden>
            <span
              className="armada-usage__fill"
              style={{
                width: `${top > 0 ? (one.share / top) * 100 : 0}%`,
                minWidth: "2px",
              }}
            />
          </span>
          <Fig tip={tip}>{pct(one.share)}</Fig>
        </li>
      ))}
    </ul>
  );
}

function DayRows({ days }: { days: ChartDay[] }) {
  const top = Math.max(...days.map((one) => one.units));
  return (
    <ul className="armada-usage__shares" aria-label="Burn by day">
      {days.map((one) => (
        <li key={one.day} aria-label={`${dayLabel(one.day)}, ${M(one.units)} million units`}>
          <span className="armada-usage__name">{dayLabel(one.day).slice(4)}</span>
          <span className="armada-usage__track" aria-hidden>
            <span
              className="armada-usage__fill"
              style={{
                width: `${top > 0 ? (one.units / top) * 100 : 0}%`,
              }}
            />
          </span>
          <Fig tip="Cost units, millions">{M(one.units)}</Fig>
        </li>
      ))}
    </ul>
  );
}

const BURN_HEIGHT = 180;

/** One line a model, in the order the models keep everywhere, so a model keeps its place. */
function burnPlot(hours: UsageHours): HourPlot {
  const folded = hours.map((one) => (one.burn === undefined ? undefined : foldModels(one.burn)));
  const names = foldModels(hours.flatMap((one) => one.burn ?? [])).map((one) => one.name);
  return {
    key: "burn",
    figure: "burn",
    height: BURN_HEIGHT,
    steps: true,
    direct: true,
    sum: "All",
    say: (value) => `${M(value)}M`,
    series: names.map((name) => ({
      key: name,
      name,
      model: name,
      values: folded.map((one) =>
        one === undefined ? undefined : (one.find((row) => row.name === name)?.units ?? 0),
      ),
    })),
  };
}

function ModelLegend({ usage }: { usage: Pick<UsageBreakdown, "by_model" | "units"> }) {
  const rows = shareRows(foldModels(usage.by_model), usage.units);
  return (
    <ul className="armada-usage__legend" aria-label="Models">
      {rows.map((one) => (
        <li key={one.name}>
          <span className="armada-usage__swatch" data-model={one.name} aria-hidden />
          <span className="armada-usage__name">{one.name}</span>
          <Fig tip="Share of the burn">{pct(one.share)}</Fig>
        </li>
      ))}
    </ul>
  );
}

function HeatMap({
  heat,
  cell,
  weekday,
  onCell,
}: {
  heat: number[][];
  cell: Cell | undefined;
  weekday: number | undefined;
  onCell: (cell: Cell) => void;
}) {
  const [hover, setHover] = useState<Cell | undefined>(undefined);
  const shown = hover ?? cell;
  const label = (c: Cell) => `${WEEKDAYS[c.d]}, ${blockLabel(c.b)}`;
  return (
    <div className="armada-usage__map">
      <div className="armada-usage__heat" role="grid" aria-label="Burn by weekday and hour">
        <div className="armada-usage__maprow" role="row">
          <span role="columnheader" aria-label="Hour" />
          {WEEKDAYS.map((name, d) => (
            <span
              key={name}
              role="columnheader"
              className="armada-usage__colhead"
              data-on={weekday === d || undefined}
            >
              {name}
            </span>
          ))}
        </div>
        {HOURS.map((hour, b) => (
          <div key={hour} className="armada-usage__maprow" role="row">
            <span role="rowheader" className="armada-usage__rowhead">
              {hour}
            </span>
            {WEEKDAYS.map((_, d) => {
              const units = heat[b]![d]!;
              const here = {
                d,
                b,
              };
              const on = cell?.d === d && cell.b === b;
              return (
                <Tooltip key={d} asChild label={`${label(here)}: ${M(units)}M units`}>
                  <div
                    role="gridcell"
                    className="armada-usage__cell"
                    data-step={stepOf(units)}
                    aria-selected={on}
                    aria-label={`${label(here)}, ${M(units)} million units`}
                    onMouseEnter={() => setHover(here)}
                    onMouseLeave={() => setHover(undefined)}
                    onClick={() => onCell(here)}
                  />
                </Tooltip>
              );
            })}
          </div>
        ))}
      </div>
      <div className="armada-usage__readout" role="status">
        {shown === undefined ? null : (
          <>
            <span className="armada-usage__name">{label(shown)}</span>
            <Fig tip="Cost units, millions, in this 3 hour block">{`${M(heat[shown.b]![shown.d]!)}M`}</Fig>
          </>
        )}
      </div>
      <ul className="armada-usage__ramp" aria-label="Scale, million units per 3 hours">
        {[
          "0",
          ...BOUNDS.map(String),
          `${BOUNDS[2]}+`,
        ].map((name, step) => (
          <li key={name}>
            <span className="armada-usage__chip" data-step={step} aria-hidden />
            <span className="armada-usage__unit">{name}</span>
          </li>
        ))}
        <li className="armada-usage__rampunit">
          <span className="armada-usage__unit">M units per 3 hours</span>
        </li>
      </ul>
    </div>
  );
}

/**
 * What the page draws until Fleet's summary has answered: the loaded layout's own shapes in skeleton bars, no text,
 * so the figures land on them without a frame left blank or anything moving. A summary with nothing in it is not this.
 */
const JOB_BARS = [
  "100%",
  "62%",
  "48%",
  "36%",
  "30%",
  "24%",
];

function PendingJobs() {
  return (
    <ul className="armada-usage__sessions" aria-hidden>
      {JOB_BARS.map((track, at) => (
        <li key={track} className="armada-usage__session">
          <span className="armada-usage__what">
            <Bar width={`${60 + ((at * 13) % 30)}%`} />
            <Skeleton className="armada-usage__skel-track" width={track} />
          </span>
          <Bar width="80%" end />
          <Bar width="80%" end />
          <Bar width="80%" end />
          <Bar width="80%" />
        </li>
      ))}
    </ul>
  );
}

const SHARE_BARS = [
  "100%",
  "30%",
  "8%",
];

function PendingShares() {
  return (
    <ul className="armada-usage__shares" aria-hidden>
      {SHARE_BARS.map((track, at) => (
        <li key={track}>
          <Bar width={`${50 + at * 15}%`} />
          <Skeleton className="armada-usage__skel-track" width={track} />
          <Bar width="80%" end />
        </li>
      ))}
    </ul>
  );
}

function PendingLegend() {
  return (
    <ul className="armada-usage__legend" aria-hidden>
      {[
        0,
        1,
        2,
      ].map((one) => (
        <li key={one}>
          <Bar width="100%" />
        </li>
      ))}
    </ul>
  );
}

function PendingMap() {
  return (
    <div className="armada-usage__map" aria-hidden>
      <div className="armada-usage__heat">
        <div className="armada-usage__maprow">
          <span />
          {WEEKDAYS.map((name) => (
            <span key={name} className="armada-usage__colhead">
              <Bar width="60%" />
            </span>
          ))}
        </div>
        {HOURS.map((hour) => (
          <div key={hour} className="armada-usage__maprow">
            <span className="armada-usage__rowhead">
              <Bar width="70%" />
            </span>
            {WEEKDAYS.map((name) => (
              <Skeleton key={name} className="armada-usage__skel-cell" />
            ))}
          </div>
        ))}
      </div>
      <div className="armada-usage__readout" />
      <div className="armada-usage__ramp">
        <Skeleton className="armada-usage__skel-ramp" width="14rem" />
      </div>
    </div>
  );
}

function PendingRules() {
  return (
    <section className="armada-usage__panel" aria-label="Usage rules">
      <Band>Usage rules</Band>
      <ul className="armada-usage__rules" aria-hidden>
        {[
          0,
          1,
          2,
          3,
        ].map((one) => (
          <li key={one} className="armada-usage__rule">
            <Skeleton className="armada-usage__skel-dot" />
            <Bar width="80%" />
            <Bar width={`${45 + one * 10}%`} />
          </li>
        ))}
      </ul>
    </section>
  );
}

function PendingTiles({ meter }: { meter: boolean }) {
  return (
    <ul className="armada-usage__tiles" aria-hidden>
      {[
        meter,
        false,
        false,
        false,
      ]
        .slice(meter ? 0 : 1)
        .map((has, at) => (
          <li key={at} className="armada-usage__tile">
            <Bar width="60%" />
            <span className="armada-usage__big">
              <Bar width="45%" />
            </span>
            {has ? <Skeleton className="armada-usage__skel-meter" /> : null}
            <span className="armada-usage__deltaslot" />
          </li>
        ))}
    </ul>
  );
}

function SessionRow({
  one,
  whole,
  top,
  cursor,
  picked,
  onPick,
}: {
  one: Row;
  whole: number;
  top: number;
  cursor: boolean;
  picked: boolean;
  onPick: () => void;
}) {
  const { session } = one;
  const subagent = `${Math.round(session.subagent_share * 100)}%`;
  const peak = `${Math.round(session.peak_context / 1000)}k`;
  const models = [
    ...new Set(
      session.models.map(
        (name) =>
          foldModels([
            {
              name,
              units: 0,
            },
          ])[0]!.name,
      ),
    ),
  ].join("+");
  return (
    <li
      role="option"
      id={`usage-${one.id}`}
      aria-selected={picked}
      aria-label={`${one.title}, ${M(one.units)} million units, ${subagent} in subagents, peak context ${peak}, ${models}`}
      data-cursor={cursor || undefined}
      className="armada-usage__session"
      onClick={onPick}
    >
      <span className="armada-usage__what">
        <Tooltip label={one.title}>
          <span className="armada-usage__title" data-id={!one.known || undefined}>
            {one.title}
          </span>
        </Tooltip>
        <span className="armada-usage__track" aria-hidden>
          <span
            className="armada-usage__fill"
            style={{
              width: `${top > 0 ? (one.units / top) * 100 : 0}%`,
            }}
          />
        </span>
      </span>
      <Fig tip="Share of the week's burn">{pct(shareOf(one.units, whole))}</Fig>
      <Fig tip="Share of this job's burn that ran in subagents">{subagent}</Fig>
      <Fig tip="Peak context" over={session.peak_context >= PEAK_OVER}>
        {peak}
      </Fig>
      <Tooltip label="Models used">
        <span className="armada-usage__model">{models}</span>
      </Tooltip>
    </li>
  );
}

export function UsagePage({
  summary,
  titles,
  planPercent,
  rules,
  time,
  builds,
  hours,
  spent,
  onBuild,
}: UsagePageProps) {
  const [view, setView] = useState<View>("tokens");
  const offered = VIEWS.filter(
    (one) =>
      one.id === "tokens" ||
      (one.id === "time" && time !== undefined) ||
      (one.id === "spent" && spent !== undefined),
  );
  const [cursor, setCursor] = useState(-1);
  const [rulePick, setRulePick] = useState(-1);
  const [pick, setPick] = useState(-1);
  const [cut, setCut] = useState<Cut>("model");
  const [cell, setCell] = useState<Cell | undefined>(undefined);
  const [hour, setHour] = useState<number | undefined>(undefined);
  const [hoverHour, setHoverHour] = useState<number | undefined>(undefined);
  const [build, setBuild] = useState<string | undefined>(undefined);

  useEffect(() => {
    onBuild?.(build);
  }, [
    build,
    onBuild,
  ]);

  const listed = builds === undefined || builds === "pending" ? undefined : builds;
  const drawn = hours === undefined || hours === "pending" ? undefined : hours;
  const pickedBuild = listed?.find((one) => one.id === build);
  const rows = useMemo(() => {
    if (summary === undefined) return [];
    const all = rowsOf(summary, titles);
    return pickedBuild === undefined ? all : all.filter((one) => pickedBuild.jobs.includes(one.id));
  }, [
    summary,
    titles,
    pickedBuild,
  ]);
  const picked = rows[pick];
  const whole = useMemo(
    () => (summary === undefined ? undefined : chartDays(summary, summary.total.by_day)),
    [
      summary,
    ],
  );
  const source: Source | undefined = useMemo(() => {
    if (summary === undefined || whole === undefined) return undefined;
    if (picked !== undefined)
      return {
        title: picked.title,
        usage: picked.session.usage,
        days: chartDays(summary, picked.session.usage.by_day),
        session: picked.session,
        scoped: true,
      };
    if (pickedBuild?.usage !== undefined)
      return {
        title: `Build ${commitOf(pickedBuild)}`,
        usage: pickedBuild.usage,
        days: chartDays(summary, pickedBuild.usage.by_day),
        scoped: true,
      };
    // Its tokens are still being read: the panels they feed draw their shapes meanwhile.
    if (pickedBuild !== undefined) return undefined;
    return {
      title: "All jobs",
      usage: summary.total,
      days: whole,
      scoped: false,
    };
  }, [
    summary,
    whole,
    picked,
    pickedBuild,
  ]);
  const heat = useMemo(
    () => heatOf(source?.usage.hour_grid ?? []),
    [
      source,
    ],
  );
  const views = useMemo(
    () => (rules === undefined ? undefined : ruleViews(rules)),
    [
      rules,
    ],
  );
  const ruleCount = views?.length ?? 0;
  const viewed = useRef(view);
  viewed.current = view;
  const latest = useRef({
    cursor,
    pick,
    cell,
    heat,
    count: rows.length,
    rulePick,
    ruleCount,
    offered: offered.map((one) => one.id),
    build,
    listed: builds !== undefined,
    hour,
    hours: drawn,
  });
  latest.current = {
    cursor,
    pick,
    cell,
    heat,
    count: rows.length,
    rulePick,
    ruleCount,
    offered: offered.map((one) => one.id),
    build,
    listed: builds !== undefined,
    hour,
    hours: drawn,
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
      const among = latest.current.offered;
      if (event.key === "v" && among.length > 1) {
        setView((now) => among[(among.indexOf(now) + 1) % among.length]!);
        return event.preventDefault();
      }
      if (event.key === "c" && latest.current.listed) {
        document.querySelector<HTMLElement>("[data-builds]")?.focus();
        return event.preventDefault();
      }
      if (inBuilds(target) && LIST_KEYS.includes(event.key)) return;
      if (viewed.current !== "tokens") return;
      const last = latest.current.count - 1;
      const onControl =
        target instanceof HTMLElement && target.closest("button, [role=tab]") !== null;
      const onTab = target instanceof HTMLElement && target.closest("[role=tab]") !== null;
      const step = (dd: number, db: number) => {
        setHour(undefined);
        setCell((now) => {
          if (now === undefined) return peakCell(latest.current.heat);
          return {
            d: Math.min(6, Math.max(0, now.d + dd)),
            b: Math.min(7, Math.max(0, now.b + db)),
          };
        });
      };
      /** An hour on the chart, and the map's cell that holds it. */
      const stepHour = (by: number) => {
        const { hours: among, hour: now } = latest.current;
        if (among === undefined) return step(by, 0);
        const next =
          now === undefined ? latestHour(among) : Math.min(SHOWN - 1, Math.max(0, now + by));
        setHour(next);
        setCell({
          d: weekdayOfHour(among, next),
          b: blockOfHour(among, next),
        });
      };
      if (event.key === "ArrowDown" || event.key === "j") setCursor((at) => Math.min(last, at + 1));
      else if (event.key === "ArrowUp" || event.key === "k") setCursor((at) => Math.max(0, at - 1));
      else if ((event.key === "Enter" || event.key === "o") && !onControl) {
        if (latest.current.cursor < 0) return;
        setPick(latest.current.cursor);
        setCell(undefined);
        setHour(undefined);
      } else if (
        event.key === "Escape" &&
        (latest.current.cell !== undefined || latest.current.hour !== undefined)
      ) {
        setCell(undefined);
        setHour(undefined);
      } else if (event.key === "Escape" && latest.current.rulePick >= 0) setRulePick(-1);
      else if (/^[1-9]$/.test(event.key) && Number(event.key) <= latest.current.ruleCount)
        setRulePick(Number(event.key) - 1);
      else if (event.key === "Escape" && latest.current.pick >= 0) setPick(-1);
      else if (event.key === "Escape" && latest.current.build !== undefined) setBuild(undefined);
      else if ((event.key === "ArrowRight" || event.key === "l") && !onTab) stepHour(1);
      else if ((event.key === "ArrowLeft" || event.key === "h") && !onTab) stepHour(-1);
      else if (event.key === "]") step(0, 1);
      else if (event.key === "[") step(0, -1);
      else if (CUT_KEY[event.key] !== undefined) setCut(CUT_KEY[event.key]!);
      else return;
      event.preventDefault();
    }
    window.addEventListener("keydown", pressed);
    return () => window.removeEventListener("keydown", pressed);
  }, []);

  useEffect(() => {
    setCursor(-1);
    setPick(-1);
    setCell(undefined);
    setHour(undefined);
  }, [
    build,
  ]);

  useEffect(() => {
    const row = rows[cursor];
    if (row !== undefined)
      document.getElementById(`usage-${row.id}`)?.scrollIntoView?.({
        block: "nearest",
      });
  }, [
    cursor,
    rows,
  ]);

  const top = Math.max(0, ...rows.map((one) => one.units));
  const active = rows[cursor];
  const shown = hoverHour ?? hour;
  const weekday =
    shown !== undefined && drawn !== undefined ? weekdayOfHour(drawn, shown) : cell?.d;
  const select = (at: number) => {
    if (drawn === undefined) return;
    setHour(at);
    setCell({
      d: weekdayOfHour(drawn, at),
      b: blockOfHour(drawn, at),
    });
  };
  const burn = useMemo(
    () => (drawn === undefined ? undefined : burnPlot(drawn)),
    [
      drawn,
    ],
  );
  const burned = useMemo(
    () =>
      drawn === undefined
        ? undefined
        : {
            by_model: foldModels(drawn.flatMap((one) => one.burn ?? [])),
            units: drawn.reduce(
              (sum, one) => sum + (one.burn ?? []).reduce((all, row) => all + row.units, 0),
              0,
            ),
          },
    [
      drawn,
    ],
  );
  const total = summary?.total.units ?? 0;
  const rest = summary?.rest;
  const pending = summary === undefined;
  const scoping =
    pending ||
    (pickedBuild !== undefined && picked === undefined && pickedBuild.usage === undefined);
  const switcher =
    offered.length < 2 ? null : (
      <div className="armada-usage__views">
        <Tabs items={offered} value={view} onChange={(id) => setView(id as View)} />
        <Kbd>v</Kbd>
      </div>
    );
  if (view === "time" && time !== undefined)
    return (
      <div className="armada-usage armada-page">
        {switcher}
        <TimeView
          time={time}
          builds={builds}
          hours={hours}
          build={pickedBuild}
          onBuild={setBuild}
        />
      </div>
    );
  if (view === "spent" && spent !== undefined)
    return (
      <div className="armada-usage armada-page">
        {switcher}
        <SpentView spent={spent} />
      </div>
    );

  return (
    <div className="armada-usage armada-page" aria-busy={pending || undefined}>
      {switcher}
      <div className="armada-usage__page">
        <div className="armada-usage__left">
          <section className="armada-usage__panel" aria-label="Job list">
            <Band
              keys={[
                "↑↓",
                "↵",
                "Esc",
              ]}
            >
              Jobs
            </Band>
            {pending ? null : (
              <div className="armada-usage__heads" aria-hidden>
                <span />
                <span>Week</span>
                <span>Subagents</span>
                <span>Peak context</span>
                <span>Models</span>
              </div>
            )}
            {pending ? (
              <PendingJobs />
            ) : (
              <ul
                className="armada-usage__sessions"
                role="listbox"
                aria-label="Jobs"
                aria-activedescendant={active === undefined ? undefined : `usage-${active.id}`}
                tabIndex={0}
              >
                {rows.map((one, at) => (
                  <SessionRow
                    key={one.id}
                    one={one}
                    whole={total}
                    top={top}
                    cursor={at === cursor}
                    picked={at === pick}
                    onPick={() => {
                      setCursor(at);
                      setPick(at);
                      setCell(undefined);
                      setHour(undefined);
                    }}
                  />
                ))}
              </ul>
            )}
            {rest === undefined ? null : (
              <div className="armada-usage__rest">
                <span className="armada-usage__what">
                  <span className="armada-usage__title">rest</span>
                  <span className="armada-usage__track" aria-hidden>
                    <span
                      className="armada-usage__fill"
                      style={{
                        width: `${top > 0 ? (rest.usage.units / top) * 100 : 0}%`,
                      }}
                    />
                  </span>
                </span>
                <Fig tip="Share of the week's burn">{pct(shareOf(rest.usage.units, total))}</Fig>
              </div>
            )}
          </section>

          {builds === undefined ? null : (
            <BuildsPanel builds={builds} view="tokens" pick={build} onPick={setBuild} />
          )}

          <section className="armada-usage__panel" aria-label="Breakdown">
            <Band
              keys={[
                "m",
                "t",
                "b",
                "y",
                "d",
              ]}
            >
              Breakdown
            </Band>
            {scoping ? (
              <>
                <div className="armada-usage__scope" aria-hidden>
                  <Bar width="8rem" />
                </div>
                <div className="armada-usage__tabs">
                  <Tabs items={CUTS} value={cut} onChange={(id) => setCut(id as Cut)} />
                </div>
                <div className="armada-usage__cut">
                  <PendingShares />
                </div>
              </>
            ) : source === undefined ? null : (
              <>
                <Scope title={source.title} />
                <div className="armada-usage__tabs">
                  <Tabs items={CUTS} value={cut} onChange={(id) => setCut(id as Cut)} />
                </div>
                <div
                  className="armada-usage__cut"
                  role="region"
                  aria-label={CUTS.find((one) => one.id === cut)!.label}
                >
                  {cut === "model" ? (
                    <ShareRows
                      label="Burn by model"
                      rows={shareRows(foldModels(source.usage.by_model), source.usage.units)}
                    />
                  ) : null}
                  {cut === "tool" ? (
                    <ShareRows
                      label="Burn by tool"
                      rows={shareRows(source.usage.by_first_tool, source.usage.units, toolLabel)}
                      tip="A turn's whole cost, mostly re-reading context, is charged to the tool the turn called"
                    />
                  ) : null}
                  {cut === "bash" ? (
                    <ShareRows
                      label="Burn by Bash use"
                      rows={shareRows(source.usage.by_bash_use, source.usage.units, bashLabel)}
                      tip="Share of the whole burn, in turns whose first tool was Bash used for this"
                    />
                  ) : null}
                  {cut === "token" ? (
                    <ShareRows
                      label="Burn by token type"
                      rows={shareRows(tokenRows(source.usage), source.usage.units)}
                      tip="Share of the burn by the kind of token charged"
                    />
                  ) : null}
                  {cut === "day" ? <DayRows days={source.days} /> : null}
                </div>
              </>
            )}
          </section>
        </div>

        <section className="armada-usage__panel armada-usage__burn" aria-label="Burn per hour">
          <Band
            keys={[
              "←→",
            ]}
          >
            Burn per hour
          </Band>
          {pending || hours === "pending" ? (
            <>
              <div className="armada-usage__scope" aria-hidden>
                <Bar width="8rem" />
              </div>
              <PendingHours
                plots={[
                  {
                    height: BURN_HEIGHT,
                  },
                ]}
              />
              <PendingLegend />
            </>
          ) : drawn === undefined || burn === undefined || burned === undefined ? null : (
            <>
              <Scope title="All jobs" unit="M units per hour" />
              <HourChart
                hours={drawn}
                plots={[
                  burn,
                ]}
                label="Burn per hour, last 48 hours, in million cost units, by model"
                builds={listed ?? []}
                pick={build}
                onPick={setBuild}
                hour={shown}
                onHover={setHoverHour}
                onSelect={select}
              />
              <HourRows
                hours={drawn}
                plots={[
                  burn,
                ]}
                label="Burn by hour"
              />
              <ModelLegend usage={burned} />
            </>
          )}
        </section>

        <div className="armada-usage__centre">
          {pending && rules === undefined ? <PendingRules /> : null}
          {rules === undefined || views === undefined ? null : (
            <UsageRules
              rules={views}
              pick={rulePick}
              onPick={setRulePick}
              titles={titles}
              job={picked?.id}
              now={Date.parse(rules.until)}
            />
          )}

          <section className="armada-usage__panel" aria-label="Day and hour">
            <Band
              keys={[
                "←→",
                "[",
                "]",
              ]}
            >
              Day and hour
            </Band>
            {scoping ? (
              <>
                <div className="armada-usage__scope" aria-hidden>
                  <Bar width="8rem" />
                </div>
                <PendingMap />
              </>
            ) : source === undefined ? null : (
              <>
                <Scope title={source.title} />
                <HeatMap
                  heat={heat}
                  cell={cell}
                  weekday={weekday}
                  onCell={(next) => {
                    setCell(next);
                    setHour(undefined);
                  }}
                />
              </>
            )}
          </section>
        </div>

        <div className="armada-usage__tilearea">
          {scoping ? (
            <PendingTiles meter={planPercent !== undefined} />
          ) : source === undefined ? null : (
            <Tiles summary={summary} source={source} planPercent={planPercent} />
          )}
        </div>
      </div>
    </div>
  );
}
