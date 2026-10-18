import { ArrowDown, ArrowUp } from "lucide-react";
import { useEffect, useMemo, useRef, useState } from "react";
import type { MouseEvent } from "react";

import { Kbd } from "../../primitives/Kbd/Kbd";
import { Tabs } from "../../primitives/Tabs/Tabs";
import { Tooltip } from "../../primitives/Tooltip/Tooltip";
import { BURN_PER_DAY, DAYS, HEAT, HOURS, PLAN_WINDOWS, PRIOR_WEEK, SESSIONS, WEEK, WEEKDAYS } from "./fixtures";
import type { Slice, UsageSession } from "./fixtures";

/**
 * Where the tokens went, laid out as the cockpit is: the Sessions that burned it on the left with one breakdown under
 * them, the burn per day and the day by hour map for the week or the Session picked in the centre, the headline figures
 * on the right. Mock only, on `fixtures.ts`.
 *
 * Keys: `↓ j` and `↑ k` move down the Sessions, `↵ o` picks the one under the cursor, `Esc` clears the hour cell and then
 * the pick, `m` `t` `b` `y` `d` set the breakdown to model, tool, Bash use, token type or day, `← → h l` step the day and `[ ]` step the 3 hour block
 * on the map (the first of these lands on the busiest cell).
 */

type Cut = "model" | "tool" | "bash" | "token" | "day";
const CUTS = [
  { id: "model", label: "By model" },
  { id: "tool", label: "By tool" },
  { id: "bash", label: "By Bash use" },
  { id: "token", label: "By token type" },
  { id: "day", label: "By day" },
];
const CUT_KEY: Record<string, Cut> = { m: "model", t: "tool", b: "bash", y: "token", d: "day" };

const M = (units: number) => units.toFixed(1);
const pct = (share: number) => (share > 0 && share < 0.1 ? "<0.1%" : `${share.toFixed(1)}%`);
const WEEK_TOTAL = BURN_PER_DAY.reduce((a, b) => a + b, 0);
const dayName = (day: number) => `${WEEKDAYS[day]} ${11 + day}`;

type Cell = { d: number; b: number };
type Source = { title: string; units: number; byModel: Slice[]; byTool: Slice[]; byBash: Slice[]; bashShare: number; byToken: Slice[]; byDay: number[]; session?: UsageSession };

/** The week's day by hour map scaled to a Session's own days: its hours are a sketch, the week's are measured. */
function heatFor(source: Source): number[][] {
  if (source.session === undefined) return HEAT;
  const sums = WEEKDAYS.map((_, d) => HEAT.reduce((a, row) => a + row[d]!, 0));
  return HEAT.map((row) => row.map((units, d) => (units / sums[d]!) * source.byDay[d]!));
}

function peakCell(heat: number[][]): Cell {
  let best: Cell = { d: 0, b: 0 };
  heat.forEach((row, b) => row.forEach((units, d) => { if (units > heat[best.b]![best.d]!) best = { d, b }; }));
  return best;
}

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
function Tile({ name, figure, tip, meter, delta }: { name: string; figure: string; tip: string; meter?: number; delta?: Delta }) {
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

function pointsDelta(now: number, before: number): Delta {
  const diff = Math.abs(now - before);
  return { up: now >= before, text: `${diff} pts`, tip: `${now >= before ? "Up" : "Down"} ${diff} points on the week before` };
}

function Tiles({ source }: { source: Source }) {
  const picked = source.session;
  const today = source.byDay[6]!;
  const yesterday = source.byDay[5]!;
  const change = yesterday > 0 ? ((today - yesterday) / yesterday) * 100 : undefined;
  const planPercent = PLAN_WINDOWS.find((one) => one.id === "7d")!.percent;
  const planShare = picked === undefined ? planPercent : Math.round(((planPercent * picked.units) / WEEK_TOTAL) * 10) / 10;
  const subagent = picked?.subagent ?? WEEK.subagentShare;
  const above = picked?.above200 ?? WEEK.above200;
  return (
    <ul className="armada-usage__tiles" aria-label="Headline figures">
      <Tile
        name="Weekly plan used"
        figure={`${planShare}%`}
        tip={picked === undefined ? "Share of the 7 day plan window used" : "Share of the 7 day plan window this Session used"}
        meter={planShare}
        delta={picked === undefined ? pointsDelta(planPercent, PRIOR_WEEK.plan) : undefined}
      />
      <Tile
        name="Today's burn"
        figure={`${M(today)}M`}
        tip="Cost units, millions, Oct 17"
        delta={change === undefined ? undefined : { up: change >= 0, text: `${Math.abs(change).toFixed(1)}%`, tip: `${change >= 0 ? "Up" : "Down"} ${Math.abs(change).toFixed(1)} percent on yesterday` }}
      />
      <Tile
        name="Burn in subagents"
        figure={`${subagent}%`}
        tip="Share of the burn that ran in subagents"
        delta={picked === undefined ? pointsDelta(WEEK.subagentShare, PRIOR_WEEK.subagent) : undefined}
      />
      <Tile
        name="Above 200k context"
        figure={`${above}%`}
        tip="Share of the burn from requests above 200k tokens of context"
        delta={picked === undefined ? pointsDelta(WEEK.above200, PRIOR_WEEK.above200) : undefined}
      />
    </ul>
  );
}

/**
 * A share of the burn per name, one bar a row, all to one scale. What the rows do not add up to is named as a last row,
 * and only when it is worth a row: `rest` is the name and `whole` the figure the rows are parts of. A row that carries
 * its commands draws them under its name, and the tooltip on the name lists them all.
 */
function ShareRows({ label, rows, rest, whole = 100, tip = "Share of the burn", restTip }: { label: string; rows: Slice[]; rest?: string; whole?: number; tip?: string; restTip?: string }) {
  const left = whole - rows.reduce((sum, one) => sum + one.share, 0);
  const all = rest !== undefined && left >= 0.05 ? [...rows, { name: rest, share: left }] : rows;
  const top = Math.max(...all.map((one) => one.share));
  const stacked = all.some((one) => one.top !== undefined);
  return (
    <ul className="armada-usage__shares" aria-label={label} data-stacked={stacked || undefined}>
      {all.map((one) => (
        <li key={one.name} aria-label={`${one.name}, ${pct(one.share)} of the burn${one.all === undefined ? "" : `, ${one.all}`}`}>
          <Tooltip label={one.all ?? one.name}>
            <span className="armada-usage__what">
              <span className="armada-usage__name">{one.name}</span>
              {one.top === undefined ? null : <span className="armada-usage__top">{one.top}</span>}
            </span>
          </Tooltip>
          <span className="armada-usage__track" aria-hidden>
            <span className="armada-usage__fill" style={{ width: `${(one.share / top) * 100}%`, minWidth: "2px" }} />
          </span>
          <Fig tip={one.name === rest ? restTip ?? tip : tip}>{pct(one.share)}</Fig>
        </li>
      ))}
    </ul>
  );
}

function DayRows({ units }: { units: number[] }) {
  const top = Math.max(...units);
  return (
    <ul className="armada-usage__shares" aria-label="Burn by day">
      {units.map((one, at) => (
        <li key={DAYS[at]} aria-label={`${DAYS[at]}, ${M(one)} million units`}>
          <span className="armada-usage__name">{DAYS[at]}</span>
          <span className="armada-usage__track" aria-hidden>
            <span className="armada-usage__fill" style={{ width: `${(one / top) * 100}%` }} />
          </span>
          <Fig tip="Cost units, millions">{M(one)}</Fig>
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
const MAX = 320;
const y = (units: number) => BASE - ((BASE - TOP) * units) / MAX;
const SLOT = (W - LEFT - RIGHT) / BURN_PER_DAY.length;
const BAR = SLOT - 18;
const barX = (at: number) => LEFT + at * SLOT + (SLOT - BAR) / 2;

/** A column from `low` to `high` units; its top corners round only when it is the top of a stack. */
const column = (x: number, low: number, high: number, round: boolean) => {
  const r = round ? Math.min(4, (y(low) - y(high)) / 2) : 0;
  return `M${x},${y(low)} V${y(high) + r} Q${x},${y(high)} ${x + r},${y(high)} H${x + BAR - r} Q${x + BAR},${y(high)} ${x + BAR},${y(high) + r} V${y(low)} Z`;
};

type Part = { name: string; low: number; high: number; units: number };

/** Stacked by model, bottom to top, always in the same order so a model keeps its place. */
const ORDER = ["sonnet", "opus", "haiku"];
function stack(total: number, rows: Slice[]): Part[] {
  let low = 0;
  return [...rows].sort((a, b) => ORDER.indexOf(a.name) - ORDER.indexOf(b.name)).map((one) => {
    const units = (total * one.share) / 100;
    const part = { name: one.name, low, high: low + units, units };
    low += units;
    return part;
  });
}

/** Under a pixel, a model is named in the tooltip only. */
const drawn = (parts: Part[]) => parts.filter((one) => y(one.low) - y(one.high) >= 1);

function Stack({ parts, at }: { parts: Part[]; at: number }) {
  const shown = drawn(parts);
  return (
    <>
      {shown.map((part, i) => (
        <path key={part.name} className="armada-usage__seg-model" data-model={part.name} d={column(barX(at), part.low, part.high, i === shown.length - 1)} />
      ))}
    </>
  );
}

function DayChart({ source, day, onDay, onPick }: { source: Source; day: number | undefined; onDay: (day: number | undefined) => void; onPick: (day: number) => void }) {
  const picked = source.session;
  const place = (event: MouseEvent<HTMLDivElement>) => {
    const box = event.currentTarget.getBoundingClientRect();
    const x = ((event.clientX - box.left) / box.width) * W;
    const at = Math.floor((x - LEFT) / SLOT);
    onDay(at >= 0 && at < BURN_PER_DAY.length ? at : undefined);
  };
  const peak = BURN_PER_DAY.indexOf(Math.max(...BURN_PER_DAY));
  const last = stack(BURN_PER_DAY[6]!, WEEK.byModel);
  return (
    <div className="armada-usage__plot" onMouseMove={place} onMouseLeave={() => onDay(undefined)}>
      <svg className="armada-usage__chart" viewBox={`0 0 ${W} ${H}`} role="img" aria-label="Burn per day, 11 to 17 October, in million cost units, by model">
        {[0, 100, 200, 300].map((tick) => (
          <g key={tick}>
            <line className="armada-usage__grid" x1={LEFT} x2={W - RIGHT} y1={y(tick)} y2={y(tick)} />
            <text className="armada-usage__tick" x={LEFT - 6} y={y(tick) + 4} textAnchor="end">{tick}</text>
          </g>
        ))}
        {day === undefined ? null : <rect className="armada-usage__daywash" x={LEFT + day * SLOT} y={TOP - 8} width={SLOT} height={BASE - TOP + 8} />}
        {BURN_PER_DAY.map((units, at) => (
          <g key={DAYS[at]} data-quiet={picked !== undefined || undefined}>
            <Stack parts={stack(units, WEEK.byModel)} at={at} />
          </g>
        ))}
        {picked === undefined
          ? null
          : BURN_PER_DAY.map((_, at) => (
              <g key={`mine-${DAYS[at]}`}>
                <Stack parts={stack(picked.byDay[at]!, picked.byModel)} at={at} />
              </g>
            ))}
        {drawn(last).map((part) => (
          <text key={part.name} className="armada-usage__direct" x={barX(6) + BAR + 6} y={(y(part.low) + y(part.high)) / 2 + 4}>{part.name}</text>
        ))}
        <text className="armada-usage__peak" x={barX(peak) + BAR / 2} y={y(BURN_PER_DAY[peak]!) - 6} textAnchor="middle">{M(BURN_PER_DAY[peak]!)}</text>
        {BURN_PER_DAY.map((_, at) => (
          <g key={DAYS[at]}>
            <text className="armada-usage__tick" data-on={day === at || undefined} x={barX(at) + BAR / 2} y={BASE + 16} textAnchor="middle">{WEEKDAYS[at]}</text>
            <text className="armada-usage__tick" data-on={day === at || undefined} x={barX(at) + BAR / 2} y={BASE + 30} textAnchor="middle">{11 + at}</text>
          </g>
        ))}
        <line className="armada-usage__base" x1={LEFT} x2={W - RIGHT} y1={BASE} y2={BASE} />
        {day === undefined ? null : <line className="armada-usage__cross" x1={barX(day) + BAR / 2} x2={barX(day) + BAR / 2} y1={TOP - 8} y2={BASE} />}
      </svg>
      <div className="armada-usage__hits" aria-hidden>
        {BURN_PER_DAY.map((_, at) => (
          <button key={DAYS[at]} type="button" tabIndex={-1} className="armada-usage__hit" style={{ left: `${((LEFT + at * SLOT) / W) * 100}%`, width: `${(SLOT / W) * 100}%` }} onClick={() => onPick(at)} />
        ))}
      </div>
      {day === undefined ? null : (
        <div
          className="armada-usage__pop"
          role="status"
          data-flip={day >= 4 || undefined}
          style={{ left: `${((barX(day) + BAR / 2) / W) * 100}%` }}
        >
          <span className="armada-usage__popday">{`${WEEKDAYS[day]} ${DAYS[day]}`}</span>
          {[...stack(BURN_PER_DAY[day]!, WEEK.byModel)].reverse().map((part) => (
            <span key={part.name} className="armada-usage__popline">
              <span className="armada-usage__swatch" data-model={part.name} aria-hidden />
              <span>{part.name}</span>
              <span className="armada-usage__fig">{`${M(part.units)}M`}</span>
            </span>
          ))}
          <span className="armada-usage__popline" data-total>
            <span />
            <span>All</span>
            <span className="armada-usage__fig">{`${M(BURN_PER_DAY[day]!)}M`}</span>
          </span>
          {picked === undefined ? null : (
            <span className="armada-usage__popline">
              <span />
              <span className="armada-usage__title">{picked.title}</span>
              <span className="armada-usage__fig">{`${M(picked.byDay[day]!)}M`}</span>
            </span>
          )}
        </div>
      )}
    </div>
  );
}

function ModelLegend({ rows }: { rows: Slice[] }) {
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

/** Upper bounds of the first four steps, units per 3 hours; a fifth step is everything above. */
const BOUNDS = [25, 50, 75];
const stepOf = (units: number) => (units <= 0 ? 0 : units <= BOUNDS[0]! ? 1 : units <= BOUNDS[1]! ? 2 : units <= BOUNDS[2]! ? 3 : 4);

function HeatMap({ heat, cell, day, onCell, onDay }: { heat: number[][]; cell: Cell | undefined; day: number | undefined; onCell: (cell: Cell) => void; onDay: (day: number | undefined) => void }) {
  const [hover, setHover] = useState<Cell | undefined>(undefined);
  const shown = hover ?? cell;
  const label = (c: Cell) => `${dayName(c.d)}, ${HOURS[c.b]} to ${String((c.b * 3 + 3) % 24).padStart(2, "0")}h`;
  return (
    <div className="armada-usage__map">
      <div className="armada-usage__heat" role="grid" aria-label="Burn by day and hour">
        <div className="armada-usage__maprow" role="row">
          <span role="columnheader" aria-label="Hour" />
          {WEEKDAYS.map((name, d) => (
            <span key={name} role="columnheader" className="armada-usage__colhead" data-on={day === d || undefined}>{`${name} ${11 + d}`}</span>
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
                    onMouseEnter={() => { setHover(here); onDay(d); }}
                    onMouseLeave={() => { setHover(undefined); onDay(undefined); }}
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

function SessionRow({ one, top, cursor, picked, onPick }: { one: UsageSession; top: number; cursor: boolean; picked: boolean; onPick: () => void }) {
  return (
    <li
      role="option"
      id={`usage-${one.id}`}
      aria-selected={picked}
      aria-label={`${one.title}, ${M(one.units)} million units, ${one.subagent}% in subagents, peak context ${one.peak}k, ${one.model}`}
      data-cursor={cursor || undefined}
      className="armada-usage__session"
      onClick={onPick}
    >
      <span className="armada-usage__what">
        <Tooltip label={one.title}>
          <span className="armada-usage__title">{one.title}</span>
        </Tooltip>
        <span className="armada-usage__track" aria-hidden>
          <span className="armada-usage__fill" style={{ width: `${(one.units / top) * 100}%` }} />
        </span>
      </span>
      <Fig tip="Share of the week's burn">{pct((one.units / WEEK_TOTAL) * 100)}</Fig>
      <Fig tip="Share of this Session's burn that ran in subagents">{`${one.subagent}%`}</Fig>
      <Fig tip="Peak context">{`${one.peak}k`}</Fig>
      <Tooltip label="Models used">
        <span className="armada-usage__model">{one.model}</span>
      </Tooltip>
    </li>
  );
}

export function UsagePage() {
  const [cursor, setCursor] = useState(-1);
  const [pick, setPick] = useState(-1);
  const [cut, setCut] = useState<Cut>("model");
  const [cell, setCell] = useState<Cell | undefined>(undefined);
  const [hoverDay, setHoverDay] = useState<number | undefined>(undefined);

  const picked = SESSIONS[pick];
  const source: Source = picked === undefined ? { title: "All sessions", units: WEEK_TOTAL, byModel: WEEK.byModel, byTool: WEEK.byTool, byBash: WEEK.byBash, bashShare: WEEK.bashShare, byToken: WEEK.byToken, byDay: BURN_PER_DAY } : { ...picked, session: picked };
  const heat = useMemo(() => heatFor(source), [pick]); // eslint-disable-line react-hooks/exhaustive-deps
  const latest = useRef({ cursor, pick, cell, heat });
  latest.current = { cursor, pick, cell, heat };

  useEffect(() => {
    function pressed(event: KeyboardEvent): void {
      if (event.metaKey || event.ctrlKey || event.altKey) return;
      if (document.querySelector('[role="dialog"], [role="alertdialog"], [role="menu"]') !== null) return;
      const target = event.target;
      if (target instanceof HTMLElement && (target.isContentEditable || ["INPUT", "TEXTAREA", "SELECT"].includes(target.tagName))) return;
      const last = SESSIONS.length - 1;
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
    const row = SESSIONS[cursor];
    if (row !== undefined) document.getElementById(`usage-${row.id}`)?.scrollIntoView?.({ block: "nearest" });
  }, [cursor]);

  const top = Math.max(...SESSIONS.map((one) => one.units));
  const active = SESSIONS[cursor];
  const day = hoverDay ?? cell?.d;
  const pickDay = (d: number) => {
    const column = heat.map((row) => row[d]!);
    setCell({ d, b: column.indexOf(Math.max(...column)) });
  };

  return (
    <div className="armada-usage armada-page">
      <h1 className="armada-page__title">Usage</h1>

      <div className="armada-usage__page">
        <div className="armada-usage__left">
          <section className="armada-usage__panel" aria-label="Session list">
            <Band keys={["↑↓", "↵", "Esc"]}>Sessions</Band>
            <div className="armada-usage__heads" aria-hidden>
              <span />
              <span>Week</span>
              <span>Subagents</span>
              <span>Peak context</span>
              <span>Models</span>
            </div>
            <ul className="armada-usage__sessions" role="listbox" aria-label="Sessions" aria-activedescendant={active === undefined ? undefined : `usage-${active.id}`} tabIndex={0}>
              {SESSIONS.map((one, at) => (
                <SessionRow key={one.id} one={one} top={top} cursor={at === cursor} picked={at === pick} onPick={() => { setCursor(at); setPick(at); setCell(undefined); }} />
              ))}
            </ul>
          </section>

          <section className="armada-usage__panel" aria-label="Breakdown">
            <Band keys={["m", "t", "b", "y", "d"]}>Breakdown</Band>
            <Scope title={source.title} />
            <div className="armada-usage__tabs">
              <Tabs items={CUTS} value={cut} onChange={(id) => setCut(id as Cut)} />
            </div>
            <div className="armada-usage__cut" role="region" aria-label={CUTS.find((one) => one.id === cut)!.label}>
              {cut === "model" ? <ShareRows label="Burn by model" rows={source.byModel} /> : null}
              {cut === "tool" ? <ShareRows label="Burn by tool" rows={source.byTool} rest="Other" tip="A turn's whole cost, mostly re-reading context, is charged to the tool the turn called" restTip="Turns whose first tool is not listed" /> : null}
              {cut === "bash" ? <ShareRows label="Burn by Bash use" rows={source.byBash} tip="Share of the whole burn, in turns whose first tool was Bash used for this" /> : null}
              {cut === "token" ? <ShareRows label="Burn by token type" rows={source.byToken} tip="Share of the burn by the kind of token charged" /> : null}
              {cut === "day" ? <DayRows units={source.byDay} /> : null}
            </div>
          </section>
        </div>

        <div className="armada-usage__centre">
          <section className="armada-usage__panel" aria-label="Burn per day">
            <Band keys={["←→"]}>Burn per day</Band>
            <Scope title={source.title} unit="M units" />
            <DayChart source={source} day={day} onDay={setHoverDay} onPick={pickDay} />
            <ModelLegend rows={source.byModel} />
          </section>

          <section className="armada-usage__panel" aria-label="Day and hour">
            <Band keys={["←→", "[", "]"]}>Day and hour</Band>
            <Scope title={source.title} />
            <HeatMap heat={heat} cell={cell} day={day} onCell={setCell} onDay={setHoverDay} />
          </section>
        </div>

        <div className="armada-usage__tilearea">
          <Tiles source={source} />
        </div>
      </div>
    </div>
  );
}
