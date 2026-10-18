import { ArrowDown, ArrowUp } from "lucide-react";
import { useEffect, useMemo, useRef, useState } from "react";
import type { MouseEvent } from "react";
import type { NamedUnits, SessionUsageSummary, UsageBreakdown, UsageSummary } from "@armada/protocol";

import { Kbd } from "../../primitives/Kbd/Kbd";
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
  stepOf,
  tickStep,
  tokenRows,
  toolLabel,
} from "./usage-view";
import type { ChartDay, Row, ShareRow } from "./usage-view";

/**
 * Where the tokens went, laid out as the cockpit is: the Jobs that burned it on the left with one breakdown under them,
 * the burn per day and the day by hour map for the week or the Job picked in the centre, the headline figures on the
 * right. It draws Fleet's usage summary, and what the summary lacks it leaves empty.
 *
 * Keys: `↓ j` and `↑ k` move down the Jobs, `↵ o` picks the one under the cursor, `Esc` clears the hour cell and then
 * the pick, `m` `t` `b` `y` `d` set the breakdown to model, tool, Bash use, token type or day, `← → h l` step the
 * weekday and `[ ]` step the 3 hour block on the map (the first of these lands on the busiest cell).
 */

export type UsagePageProps = {
  /** Fleet's summary of the last seven days. Absent until it has answered: the panels draw empty. */
  summary?: UsageSummary | undefined;
  /** The titles Bridge holds for Jobs, by id. A Job with none shows its short id. */
  titles?: ReadonlyMap<string, string> | undefined;
  /** The 7 day plan window's percent used, from the plan's own reading. Absent draws no plan tile. */
  planPercent?: number | undefined;
};

type Cut = "model" | "tool" | "bash" | "token" | "day";
const CUTS = [
  { id: "model", label: "By model" },
  { id: "tool", label: "By tool" },
  { id: "bash", label: "By Bash use" },
  { id: "token", label: "By token type" },
  { id: "day", label: "By day" },
];
const CUT_KEY: Record<string, Cut> = { m: "model", t: "tool", b: "bash", y: "token", d: "day" };

type Cell = { d: number; b: number };
type Source = { title: string; usage: UsageBreakdown; days: ChartDay[]; session?: SessionUsageSummary };

function Band({ children, keys }: { children: string; keys?: string[] }) {
  return (
    <div className="armada-usage__band">
      <span>{children}</span>
      {keys === undefined ? null : (
        <span className="armada-usage__keys">
          {keys.map((one) => (
            <Kbd key={one}>{one}</Kbd>
          ))}
        </span>
      )}
    </div>
  );
}

function Fig({ tip, children }: { tip: string; children: string }) {
  return (
    <Tooltip label={tip}>
      <span className="armada-usage__fig">{children}</span>
    </Tooltip>
  );
}

function Scope({ title, unit }: { title: string; unit?: string }) {
  return (
    <div className="armada-usage__scope">
      <span className="armada-usage__title">{title}</span>
      {unit === undefined ? null : <span className="armada-usage__unit">{unit}</span>}
    </div>
  );
}

type Delta = { up: boolean; text: string; tip: string };

/** One headline figure, a delta against the period before, and a thin meter where the figure is a share of a limit. */
function Tile({ name, figure, tip, meter, delta }: { name: string; figure: string; tip: string; meter?: number; delta?: Delta | undefined }) {
  return (
    <li className="armada-usage__tile" aria-label={name}>
      <span className="armada-usage__name">{name}</span>
      <span className="armada-usage__big">
        <Fig tip={tip}>{figure}</Fig>
      </span>
      {meter === undefined ? null : (
        <span className="armada-usage__meter" role="meter" aria-label={name} aria-valuemin={0} aria-valuemax={100} aria-valuenow={meter}>
          <span className="armada-usage__fill" style={{ width: `${meter}%` }} />
        </span>
      )}
      <span className="armada-usage__deltaslot">
        {delta === undefined ? null : (
          <Tooltip label={delta.tip}>
            <span className="armada-usage__delta" aria-label={delta.tip}>
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
  return moved === undefined ? undefined : { up: moved.up, text: `${moved.points} pts`, tip: `${moved.up ? "Up" : "Down"} ${moved.points} points on the week before` };
}

function Tiles({ summary, source, planPercent }: { summary: UsageSummary; source: Source; planPercent: number | undefined }) {
  const { usage, session } = source;
  const today = source.days[6]!.units;
  const change = percentDelta(today, source.days[5]!.units);
  const planShare = planPercent === undefined ? undefined : session === undefined ? planPercent : Math.round(shareOf(usage.units, summary.total.units) * planPercent) / 100;
  const hadBefore = summary.prior.units > 0;
  const subagent = shareOf(usage.subagent_units, usage.units);
  const above = shareOf(usage.above_200k_units, usage.units);
  return (
    <ul className="armada-usage__tiles" aria-label="Headline figures">
      {planShare === undefined ? null : (
        <Tile
          name="Weekly plan used"
          figure={`${Math.round(planShare * 10) / 10}%`}
          tip={session === undefined ? "Share of the 7 day plan window used" : "Share of the 7 day plan window this job used"}
          meter={planShare}
        />
      )}
      <Tile
        name="Today's burn"
        figure={`${M(today)}M`}
        tip={`Cost units, millions, ${dayLabel(source.days[6]!.day)}`}
        delta={change === undefined ? undefined : { up: change.up, text: `${change.percent.toFixed(1)}%`, tip: `${change.up ? "Up" : "Down"} ${change.percent.toFixed(1)} percent on yesterday` }}
      />
      <Tile
        name="Burn in subagents"
        figure={`${Math.round(subagent)}%`}
        tip="Share of the burn that ran in subagents"
        delta={session === undefined ? points(subagent, summary.prior.subagent_share * 100, hadBefore) : undefined}
      />
      <Tile
        name="Above 200k context"
        figure={`${Math.round(above)}%`}
        tip="Share of the burn from requests above 200k tokens of context"
        delta={session === undefined ? points(above, summary.prior.above_200k_share * 100, hadBefore) : undefined}
      />
    </ul>
  );
}

/** A share of the burn per name, one bar a row, all to one scale. */
function ShareRows({ label, rows, tip = "Share of the burn" }: { label: string; rows: ShareRow[]; tip?: string }) {
  const top = Math.max(0, ...rows.map((one) => one.share));
  return (
    <ul className="armada-usage__shares" aria-label={label}>
      {rows.map((one) => (
        <li key={one.name} aria-label={`${one.name}, ${pct(one.share)} of the burn`}>
          <Tooltip label={one.name}>
            <span className="armada-usage__name">{one.name}</span>
          </Tooltip>
          <span className="armada-usage__track" aria-hidden>
            <span className="armada-usage__fill" style={{ width: `${top > 0 ? (one.share / top) * 100 : 0}%`, minWidth: "2px" }} />
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
            <span className="armada-usage__fill" style={{ width: `${top > 0 ? (one.units / top) * 100 : 0}%` }} />
          </span>
          <Fig tip="Cost units, millions">{M(one.units)}</Fig>
        </li>
      ))}
    </ul>
  );
}

const W = 520;
const H = 226;
const LEFT = 32;
const RIGHT = 48;
const TOP = 16;
const BASE = 196;
const SLOT = (W - LEFT - RIGHT) / 7;
const BAR = SLOT - 18;
const barX = (at: number) => LEFT + at * SLOT + (SLOT - BAR) / 2;

type Part = { name: string; low: number; high: number; units: number };

/** Stacked by model, bottom to top, always in the same order so a model keeps its place. */
function stack(byModel: readonly NamedUnits[]): Part[] {
  let low = 0;
  return foldModels(byModel).map((one) => {
    const part = { name: one.name, low, high: low + one.units, units: one.units };
    low += one.units;
    return part;
  });
}

type ChartProps = {
  days: ChartDay[];
  /** The picked Job's own days, drawn at full strength over the dimmed whole. */
  mine: ChartDay[] | undefined;
  mineName: string;
  day: number | undefined;
  onDay: (day: number | undefined) => void;
  onPick: (day: number) => void;
};

function DayChart({ days, mine, mineName, day, onDay, onPick }: ChartProps) {
  const peakUnits = Math.max(...days.map((one) => one.units));
  const step = tickStep(peakUnits);
  const top = Math.max(step, Math.ceil(peakUnits / step) * step);
  const y = (units: number) => BASE - ((BASE - TOP) * units) / top;
  const ticks = Array.from({ length: Math.round(top / step) + 1 }, (_, i) => i * step);
  /** A column from `low` to `high` units; its top corners round only when it is the top of a stack. */
  const column = (x: number, low: number, high: number, round: boolean) => {
    const r = round ? Math.min(4, (y(low) - y(high)) / 2) : 0;
    return `M${x},${y(low)} V${y(high) + r} Q${x},${y(high)} ${x + r},${y(high)} H${x + BAR - r} Q${x + BAR},${y(high)} ${x + BAR},${y(high) + r} V${y(low)} Z`;
  };
  /** Under a pixel, a model is named in the tooltip only. */
  const drawn = (parts: Part[]) => parts.filter((one) => y(one.low) - y(one.high) >= 1);
  const paths = (parts: Part[], at: number) => {
    const shown = drawn(parts);
    return shown.map((part, i) => <path key={part.name} className="armada-usage__seg-model" data-model={part.name} d={column(barX(at), part.low, part.high, i === shown.length - 1)} />);
  };
  const place = (event: MouseEvent<HTMLDivElement>) => {
    const box = event.currentTarget.getBoundingClientRect();
    const x = ((event.clientX - box.left) / box.width) * W;
    const at = Math.floor((x - LEFT) / SLOT);
    onDay(at >= 0 && at < days.length ? at : undefined);
  };
  const peak = peakUnits > 0 ? days.findIndex((one) => one.units === peakUnits) : -1;
  const last = stack(days[6]!.byModel);
  const shown = day === undefined ? undefined : days[day];
  const mineShown = day === undefined ? undefined : mine?.[day];
  return (
    <div className="armada-usage__plot" onMouseMove={place} onMouseLeave={() => onDay(undefined)}>
      <svg className="armada-usage__chart" viewBox={`0 0 ${W} ${H}`} role="img" aria-label="Burn per day, last seven days, in million cost units, by model">
        {ticks.map((tick) => (
          <g key={tick}>
            <line className="armada-usage__grid" x1={LEFT} x2={W - RIGHT} y1={y(tick)} y2={y(tick)} />
            <text className="armada-usage__tick" x={LEFT - 6} y={y(tick) + 4} textAnchor="end">{tick}</text>
          </g>
        ))}
        {day === undefined ? null : <rect className="armada-usage__daywash" x={LEFT + day * SLOT} y={TOP - 8} width={SLOT} height={BASE - TOP + 8} />}
        {days.map((one, at) => (
          <g key={one.day} data-quiet={mine !== undefined || undefined}>{paths(stack(one.byModel), at)}</g>
        ))}
        {mine === undefined ? null : mine.map((one, at) => <g key={`mine-${one.day}`}>{paths(stack(one.byModel), at)}</g>)}
        {drawn(last).map((part) => (
          <text key={part.name} className="armada-usage__direct" x={barX(6) + BAR + 6} y={(y(part.low) + y(part.high)) / 2 + 4}>{part.name}</text>
        ))}
        {peak < 0 ? null : <text className="armada-usage__peak" x={barX(peak) + BAR / 2} y={y(peakUnits) - 6} textAnchor="middle">{M(peakUnits)}</text>}
        {days.map((one, at) => (
          <g key={one.day}>
            <text className="armada-usage__tick" data-on={day === at || undefined} x={barX(at) + BAR / 2} y={BASE + 16} textAnchor="middle">{WEEKDAYS[one.weekday]}</text>
            <text className="armada-usage__tick" data-on={day === at || undefined} x={barX(at) + BAR / 2} y={BASE + 30} textAnchor="middle">{Number(one.day.slice(8))}</text>
          </g>
        ))}
        <line className="armada-usage__base" x1={LEFT} x2={W - RIGHT} y1={BASE} y2={BASE} />
        {day === undefined ? null : <line className="armada-usage__cross" x1={barX(day) + BAR / 2} x2={barX(day) + BAR / 2} y1={TOP - 8} y2={BASE} />}
      </svg>
      <div className="armada-usage__hits" aria-hidden>
        {days.map((one, at) => (
          <button key={one.day} type="button" tabIndex={-1} className="armada-usage__hit" style={{ left: `${((LEFT + at * SLOT) / W) * 100}%`, width: `${(SLOT / W) * 100}%` }} onClick={() => onPick(at)} />
        ))}
      </div>
      {shown === undefined || day === undefined ? null : (
        <div className="armada-usage__pop" role="status" data-flip={day >= 4 || undefined} style={{ left: `${((barX(day) + BAR / 2) / W) * 100}%` }}>
          <span className="armada-usage__popday">{dayLabel(shown.day)}</span>
          {[...stack(shown.byModel)].reverse().map((part) => (
            <span key={part.name} className="armada-usage__popline">
              <span className="armada-usage__swatch" data-model={part.name} aria-hidden />
              <span>{part.name}</span>
              <span className="armada-usage__fig">{`${M(part.units)}M`}</span>
            </span>
          ))}
          <span className="armada-usage__popline" data-total>
            <span />
            <span>All</span>
            <span className="armada-usage__fig">{`${M(shown.units)}M`}</span>
          </span>
          {mineShown === undefined ? null : (
            <span className="armada-usage__popline">
              <span />
              <span className="armada-usage__title">{mineName}</span>
              <span className="armada-usage__fig">{`${M(mineShown.units)}M`}</span>
            </span>
          )}
        </div>
      )}
    </div>
  );
}

function ModelLegend({ usage }: { usage: UsageBreakdown }) {
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

function HeatMap({ heat, cell, weekday, onCell, onWeekday }: { heat: number[][]; cell: Cell | undefined; weekday: number | undefined; onCell: (cell: Cell) => void; onWeekday: (weekday: number | undefined) => void }) {
  const [hover, setHover] = useState<Cell | undefined>(undefined);
  const shown = hover ?? cell;
  const label = (c: Cell) => `${WEEKDAYS[c.d]}, ${blockLabel(c.b)}`;
  return (
    <div className="armada-usage__map">
      <div className="armada-usage__heat" role="grid" aria-label="Burn by weekday and hour">
        <div className="armada-usage__maprow" role="row">
          <span role="columnheader" aria-label="Hour" />
          {WEEKDAYS.map((name, d) => (
            <span key={name} role="columnheader" className="armada-usage__colhead" data-on={weekday === d || undefined}>{name}</span>
          ))}
        </div>
        {HOURS.map((hour, b) => (
          <div key={hour} className="armada-usage__maprow" role="row">
            <span role="rowheader" className="armada-usage__rowhead">{hour}</span>
            {WEEKDAYS.map((_, d) => {
              const units = heat[b]![d]!;
              const here = { d, b };
              const on = cell?.d === d && cell.b === b;
              return (
                <Tooltip key={d} asChild label={`${label(here)}: ${M(units)}M units`}>
                  <div
                    role="gridcell"
                    className="armada-usage__cell"
                    data-step={stepOf(units)}
                    aria-selected={on}
                    aria-label={`${label(here)}, ${M(units)} million units`}
                    onMouseEnter={() => { setHover(here); onWeekday(d); }}
                    onMouseLeave={() => { setHover(undefined); onWeekday(undefined); }}
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
        {["0", ...BOUNDS.map(String), `${BOUNDS[2]}+`].map((name, step) => (
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

function SessionRow({ one, whole, top, cursor, picked, onPick }: { one: Row; whole: number; top: number; cursor: boolean; picked: boolean; onPick: () => void }) {
  const { session } = one;
  const subagent = `${Math.round(session.subagent_share * 100)}%`;
  const peak = `${Math.round(session.peak_context / 1000)}k`;
  const models = [...new Set(session.models.map((name) => foldModels([{ name, units: 0 }])[0]!.name))].join("+");
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
          <span className="armada-usage__title" data-id={!one.known || undefined}>{one.title}</span>
        </Tooltip>
        <span className="armada-usage__track" aria-hidden>
          <span className="armada-usage__fill" style={{ width: `${top > 0 ? (one.units / top) * 100 : 0}%` }} />
        </span>
      </span>
      <Fig tip="Share of the week's burn">{pct(shareOf(one.units, whole))}</Fig>
      <Fig tip="Share of this job's burn that ran in subagents">{subagent}</Fig>
      <Fig tip="Peak context">{peak}</Fig>
      <Tooltip label="Models used">
        <span className="armada-usage__model">{models}</span>
      </Tooltip>
    </li>
  );
}

export function UsagePage({ summary, titles, planPercent }: UsagePageProps) {
  const [cursor, setCursor] = useState(-1);
  const [pick, setPick] = useState(-1);
  const [cut, setCut] = useState<Cut>("model");
  const [cell, setCell] = useState<Cell | undefined>(undefined);
  const [hoverDay, setHoverDay] = useState<number | undefined>(undefined);

  const rows = useMemo(() => (summary === undefined ? [] : rowsOf(summary, titles)), [summary, titles]);
  const picked = rows[pick];
  const whole = useMemo(() => (summary === undefined ? undefined : chartDays(summary, summary.total.by_day)), [summary]);
  const source: Source | undefined = useMemo(() => {
    if (summary === undefined || whole === undefined) return undefined;
    if (picked === undefined) return { title: "All jobs", usage: summary.total, days: whole };
    return { title: picked.title, usage: picked.session.usage, days: chartDays(summary, picked.session.usage.by_day), session: picked.session };
  }, [summary, whole, picked]);
  const heat = useMemo(() => heatOf(source?.usage.hour_grid ?? []), [source]);
  const latest = useRef({ cursor, pick, cell, heat, count: rows.length });
  latest.current = { cursor, pick, cell, heat, count: rows.length };

  useEffect(() => {
    function pressed(event: KeyboardEvent): void {
      if (event.metaKey || event.ctrlKey || event.altKey) return;
      if (document.querySelector('[role="dialog"], [role="alertdialog"], [role="menu"]') !== null) return;
      const target = event.target;
      if (target instanceof HTMLElement && (target.isContentEditable || ["INPUT", "TEXTAREA", "SELECT"].includes(target.tagName))) return;
      const last = latest.current.count - 1;
      const onControl = target instanceof HTMLElement && target.closest("button, [role=tab]") !== null;
      const onTab = target instanceof HTMLElement && target.closest("[role=tab]") !== null;
      const step = (dd: number, db: number) =>
        setCell((now) => {
          if (now === undefined) return peakCell(latest.current.heat);
          return { d: Math.min(6, Math.max(0, now.d + dd)), b: Math.min(7, Math.max(0, now.b + db)) };
        });
      if (event.key === "ArrowDown" || event.key === "j") setCursor((at) => Math.min(last, at + 1));
      else if (event.key === "ArrowUp" || event.key === "k") setCursor((at) => Math.max(0, at - 1));
      else if ((event.key === "Enter" || event.key === "o") && !onControl) {
        if (latest.current.cursor < 0) return;
        setPick(latest.current.cursor);
        setCell(undefined);
      } else if (event.key === "Escape" && latest.current.cell !== undefined) setCell(undefined);
      else if (event.key === "Escape" && latest.current.pick >= 0) setPick(-1);
      else if ((event.key === "ArrowRight" || event.key === "l") && !onTab) step(1, 0);
      else if ((event.key === "ArrowLeft" || event.key === "h") && !onTab) step(-1, 0);
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
    const row = rows[cursor];
    if (row !== undefined) document.getElementById(`usage-${row.id}`)?.scrollIntoView?.({ block: "nearest" });
  }, [cursor, rows]);

  const top = Math.max(0, ...rows.map((one) => one.units));
  const active = rows[cursor];
  const days = whole;
  const fromCell = cell === undefined || days === undefined ? -1 : days.findIndex((one) => one.weekday === cell.d);
  const day = hoverDay ?? (fromCell < 0 ? undefined : fromCell);
  const weekday = day === undefined || days === undefined ? undefined : days[day]?.weekday;
  const hoverWeekday = (d: number | undefined) => setHoverDay(d === undefined || days === undefined ? undefined : days.findIndex((one) => one.weekday === d));
  const pickDay = (at: number) => {
    if (days === undefined) return;
    const column = days[at]!.weekday;
    const values = heat.map((row) => row[column]!);
    setCell({ d: column, b: values.indexOf(Math.max(...values)) });
  };
  const total = summary?.total.units ?? 0;
  const rest = summary?.rest;

  return (
    <div className="armada-usage armada-page">
      <h1 className="armada-page__title">Usage</h1>

      <div className="armada-usage__page">
        <div className="armada-usage__left">
          <section className="armada-usage__panel" aria-label="Job list">
            <Band keys={["↑↓", "↵", "Esc"]}>Jobs</Band>
            <div className="armada-usage__heads" aria-hidden>
              <span />
              <span>Week</span>
              <span>Subagents</span>
              <span>Peak context</span>
              <span>Models</span>
            </div>
            <ul className="armada-usage__sessions" role="listbox" aria-label="Jobs" aria-activedescendant={active === undefined ? undefined : `usage-${active.id}`} tabIndex={0}>
              {rows.map((one, at) => (
                <SessionRow key={one.id} one={one} whole={total} top={top} cursor={at === cursor} picked={at === pick} onPick={() => { setCursor(at); setPick(at); setCell(undefined); }} />
              ))}
            </ul>
            {rest === undefined ? null : (
              <div className="armada-usage__rest">
                <span className="armada-usage__what">
                  <span className="armada-usage__title">rest</span>
                  <span className="armada-usage__track" aria-hidden>
                    <span className="armada-usage__fill" style={{ width: `${top > 0 ? (rest.usage.units / top) * 100 : 0}%` }} />
                  </span>
                </span>
                <Fig tip="Share of the week's burn">{pct(shareOf(rest.usage.units, total))}</Fig>
              </div>
            )}
          </section>

          <section className="armada-usage__panel" aria-label="Breakdown">
            <Band keys={["m", "t", "b", "y", "d"]}>Breakdown</Band>
            {source === undefined ? null : (
              <>
                <Scope title={source.title} />
                <div className="armada-usage__tabs">
                  <Tabs items={CUTS} value={cut} onChange={(id) => setCut(id as Cut)} />
                </div>
                <div className="armada-usage__cut" role="region" aria-label={CUTS.find((one) => one.id === cut)!.label}>
                  {cut === "model" ? <ShareRows label="Burn by model" rows={shareRows(foldModels(source.usage.by_model), source.usage.units)} /> : null}
                  {cut === "tool" ? (
                    <ShareRows label="Burn by tool" rows={shareRows(source.usage.by_first_tool, source.usage.units, toolLabel)} tip="A turn's whole cost, mostly re-reading context, is charged to the tool the turn called" />
                  ) : null}
                  {cut === "bash" ? <ShareRows label="Burn by Bash use" rows={shareRows(source.usage.by_bash_use, source.usage.units, bashLabel)} tip="Share of the whole burn, in turns whose first tool was Bash used for this" /> : null}
                  {cut === "token" ? <ShareRows label="Burn by token type" rows={shareRows(tokenRows(source.usage), source.usage.units)} tip="Share of the burn by the kind of token charged" /> : null}
                  {cut === "day" ? <DayRows days={source.days} /> : null}
                </div>
              </>
            )}
          </section>
        </div>

        <div className="armada-usage__centre">
          <section className="armada-usage__panel" aria-label="Burn per day">
            <Band keys={["←→"]}>Burn per day</Band>
            {source === undefined || days === undefined ? null : (
              <>
                <Scope title={source.title} unit="M units" />
                <DayChart days={days} mine={source.session === undefined ? undefined : source.days} mineName={source.title} day={day} onDay={setHoverDay} onPick={pickDay} />
                <ModelLegend usage={source.usage} />
              </>
            )}
          </section>

          <section className="armada-usage__panel" aria-label="Day and hour">
            <Band keys={["←→", "[", "]"]}>Day and hour</Band>
            {source === undefined ? null : (
              <>
                <Scope title={source.title} />
                <HeatMap heat={heat} cell={cell} weekday={weekday} onCell={setCell} onWeekday={hoverWeekday} />
              </>
            )}
          </section>
        </div>

        <div className="armada-usage__tilearea">{summary === undefined || source === undefined ? null : <Tiles summary={summary} source={source} planPercent={planPercent} />}</div>
      </div>
    </div>
  );
}
