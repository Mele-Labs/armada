import { useEffect, useRef, useState } from "react";

import { Kbd } from "../../primitives/Kbd/Kbd";
import { Tabs } from "../../primitives/Tabs/Tabs";
import { Tooltip } from "../../primitives/Tooltip/Tooltip";
import { BURN_PER_DAY, DAYS, PLAN_WINDOWS, SESSIONS, WEEK } from "./fixtures";
import type { Slice, UsageSession } from "./fixtures";

/**
 * Where the tokens went: the plan's windows, the burn by day, the Sessions that burned it, and one breakdown of the week
 * or of the Session picked. Mock only, on `fixtures.ts`.
 *
 * Keys: `↓ j` and `↑ k` move down the Sessions, `↵ o` picks the one under the cursor, `Esc` goes back to the whole week,
 * `m` `t` `d` set the breakdown to model, tool or day.
 */

type Cut = "model" | "tool" | "day";
const CUTS = [
  { id: "model", label: "By model" },
  { id: "tool", label: "By tool" },
  { id: "day", label: "By day" },
];
const CUT_KEY: Record<string, Cut> = { m: "model", t: "tool", d: "day" };

const M = (units: number) => units.toFixed(1);
const pct = (share: number) => `${share.toFixed(1)}%`;
const WEEK_TOTAL = BURN_PER_DAY.reduce((a, b) => a + b, 0);

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

/** A share of the burn per name, one bar a row, all to one scale. */
function ShareRows({ label, rows, remainder }: { label: string; rows: Slice[]; remainder?: boolean }) {
  const used = rows.reduce((sum, one) => sum + one.share, 0);
  const all = remainder === true ? [...rows, { name: "Other", share: Math.max(0, 100 - used) }] : rows;
  const top = Math.max(...rows.map((one) => one.share));
  return (
    <ul className="armada-usage__shares" aria-label={label}>
      {all.map((one) => (
        <li key={one.name} aria-label={`${one.name}, ${pct(one.share)} of the burn`} data-other={one.name === "Other" || undefined}>
          <span className="armada-usage__name">{one.name}</span>
          <span className="armada-usage__track" aria-hidden>
            {one.name === "Other" ? null : <span className="armada-usage__fill" style={{ width: `${(one.share / top) * 100}%` }} />}
          </span>
          <Fig tip={one.name === "Other" ? "Turns with no tool, or one not listed" : "Share of the burn"}>{pct(one.share)}</Fig>
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

const W = 640;
const H = 176;
const LEFT = 34;
const TOP = 18;
const BASE = 150;
const MAX = 320;
const y = (units: number) => BASE - ((BASE - TOP) * units) / MAX;
/** A bar with its top corners rounded and its foot on the baseline. */
const bar = (x: number, width: number, units: number) => {
  const r = Math.min(4, (BASE - y(units)) / 2);
  return `M${x},${BASE} V${y(units) + r} Q${x},${y(units)} ${x + r},${y(units)} H${x + width - r} Q${x + width},${y(units)} ${x + width},${y(units) + r} V${BASE} Z`;
};

function DayChart({ picked }: { picked: UsageSession | undefined }) {
  const slot = (W - LEFT - 8) / BURN_PER_DAY.length;
  const width = slot - 14;
  const peak = BURN_PER_DAY.indexOf(Math.max(...BURN_PER_DAY));
  return (
    <svg className="armada-usage__chart" viewBox={`0 0 ${W} ${H}`} role="img" aria-label="Burn per day, 11 to 17 October, in million cost units">
      {[0, 100, 200, 300].map((tick) => (
        <g key={tick}>
          <line className="armada-usage__grid" x1={LEFT} x2={W - 8} y1={y(tick)} y2={y(tick)} />
          <text className="armada-usage__tick" x={LEFT - 6} y={y(tick) + 4} textAnchor="end">{tick}</text>
        </g>
      ))}
      {BURN_PER_DAY.map((units, at) => {
        const x = LEFT + at * slot + 7;
        const mine = picked?.byDay[at];
        return (
          <g key={DAYS[at]}>
            <path className="armada-usage__bar" data-quiet={picked !== undefined || undefined} d={bar(x, width, units)} />
            {mine === undefined || mine <= 0 ? null : <path className="armada-usage__bar" d={bar(x, width, mine)} />}
            {at === peak ? <text className="armada-usage__peak" x={x + width / 2} y={y(units) - 6} textAnchor="middle">{M(units)}</text> : null}
            <text className="armada-usage__tick" x={x + width / 2} y={BASE + 18} textAnchor="middle">{DAYS[at]}</text>
            <Tooltip label={`${DAYS[at]}: ${M(units)} million units${mine === undefined ? "" : `, ${M(mine)} in the pick`}`}>
              <rect className="armada-usage__hit" x={x - 7} y={TOP} width={slot} height={BASE - TOP + 24} />
            </Tooltip>
          </g>
        );
      })}
      <line className="armada-usage__base" x1={LEFT} x2={W - 8} y1={BASE} y2={BASE} />
    </svg>
  );
}

function Meters() {
  return (
    <ul className="armada-usage__meters" aria-label="Plan windows">
      {PLAN_WINDOWS.map((one) => (
        <li key={one.id}>
          <Tooltip label={one.label}>
            <span className="armada-usage__name">{one.id}</span>
          </Tooltip>
          <span className="armada-usage__meter" role="meter" aria-label={one.label} aria-valuemin={0} aria-valuemax={100} aria-valuenow={one.percent}>
            <span className="armada-usage__fill" style={{ width: `${one.percent}%` }} />
          </span>
          <Fig tip={`Used of the ${one.label}`}>{`${one.percent}%`}</Fig>
          {one.resets === undefined ? <span /> : <Fig tip="Resets">{one.resets}</Fig>}
        </li>
      ))}
    </ul>
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
      <span className="armada-usage__title">{one.title}</span>
      <span className="armada-usage__track" aria-hidden>
        <span className="armada-usage__fill" style={{ width: `${(one.units / top) * 100}%` }} />
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
  const latest = useRef({ cursor, pick });
  latest.current = { cursor, pick };

  useEffect(() => {
    function pressed(event: KeyboardEvent): void {
      if (event.metaKey || event.ctrlKey || event.altKey) return;
      if (document.querySelector('[role="dialog"], [role="alertdialog"], [role="menu"]') !== null) return;
      const target = event.target;
      if (target instanceof HTMLElement && (target.isContentEditable || ["INPUT", "TEXTAREA", "SELECT"].includes(target.tagName))) return;
      const last = SESSIONS.length - 1;
      const onControl = target instanceof HTMLElement && target.closest("button, [role=tab]") !== null;
      if (event.key === "ArrowDown" || event.key === "j") setCursor((at) => Math.min(last, at + 1));
      else if (event.key === "ArrowUp" || event.key === "k") setCursor((at) => Math.max(0, at - 1));
      else if ((event.key === "Enter" || event.key === "o") && !onControl) {
        if (latest.current.cursor < 0) return;
        setPick(latest.current.cursor);
      } else if (event.key === "Escape" && latest.current.pick >= 0) setPick(-1);
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

  const picked = SESSIONS[pick];
  const top = Math.max(...SESSIONS.map((one) => one.units));
  const source = picked ?? { title: "All sessions", units: BURN_PER_DAY.reduce((a, b) => a + b, 0), byModel: WEEK.byModel, byTool: WEEK.byTool, byDay: BURN_PER_DAY };
  const active = SESSIONS[cursor];

  return (
    <div className="armada-usage armada-page">
      <h1 className="armada-page__title">Usage</h1>

      <div className="armada-usage__row" data-wide="top">
        <section className="armada-usage__panel" aria-label="Plan">
          <Band>Plan</Band>
          <Meters />
        </section>
        <section className="armada-usage__panel" aria-label="Burn per day">
          <Band>Burn per day</Band>
          <DayChart picked={picked} />
        </section>
      </div>

      <div className="armada-usage__row" data-wide="mid">
        <section className="armada-usage__panel" aria-label="Session list">
          <Band keys={["↑↓", "↵", "Esc"]}>Sessions</Band>
          <div className="armada-usage__heads" aria-hidden>
            <span />
            <span />
            <span>Week</span>
            <span>Subagents</span>
            <span>Peak context</span>
            <span>Models</span>
          </div>
          <ul className="armada-usage__sessions" role="listbox" aria-label="Sessions" aria-activedescendant={active === undefined ? undefined : `usage-${active.id}`} tabIndex={0}>
            {SESSIONS.map((one, at) => (
              <SessionRow key={one.id} one={one} top={top} cursor={at === cursor} picked={at === pick} onPick={() => { setCursor(at); setPick(at); }} />
            ))}
          </ul>
        </section>
        <section className="armada-usage__panel" aria-label="Breakdown">
          <Band keys={["m", "t", "d"]}>Breakdown</Band>
          <div className="armada-usage__scope">
            <span className="armada-usage__title">{source.title}</span>
            <Fig tip="Share of the week's burn">{pct((source.units / WEEK_TOTAL) * 100)}</Fig>
          </div>
          <div className="armada-usage__tabs">
            <Tabs items={CUTS} value={cut} onChange={(id) => setCut(id as Cut)} />
          </div>
          <div className="armada-usage__cut" role="region" aria-label={CUTS.find((one) => one.id === cut)!.label}>
            {cut === "model" ? <ShareRows label="Burn by model" rows={source.byModel} /> : null}
            {cut === "tool" ? <ShareRows label="Burn by tool" rows={source.byTool} remainder /> : null}
            {cut === "day" ? <DayRows units={source.byDay} /> : null}
          </div>
        </section>
      </div>

      <div className="armada-usage__row" data-wide="foot">
        <section className="armada-usage__panel" aria-label="Main and subagents">
          <Band>Main and subagents</Band>
          <div className="armada-usage__split" role="img" aria-label={`Main ${WEEK.mainShare}%, subagents ${WEEK.subagentShare}%`}>
            <Tooltip label="Share of the burn in the main thread">
              <span className="armada-usage__seg" data-kind="main" style={{ flexGrow: WEEK.mainShare }}>{`main ${WEEK.mainShare}%`}</span>
            </Tooltip>
            <Tooltip label="Share of the burn in subagents">
              <span className="armada-usage__seg" data-kind="sub" style={{ flexGrow: WEEK.subagentShare }}>{`subagents ${WEEK.subagentShare}%`}</span>
            </Tooltip>
          </div>
        </section>
        <section className="armada-usage__panel" aria-label="Context size">
          <Band>Context size</Band>
          <ShareRows label="Burn by context size" rows={WEEK.contextAbove} />
        </section>
      </div>
    </div>
  );
}
