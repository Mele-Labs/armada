import { useEffect, useRef, useState } from "react";

import { Tabs } from "../../primitives/Tabs/Tabs";
import { Tooltip } from "../../primitives/Tooltip/Tooltip";
import { Band, Fig, Scope } from "./usage-parts";
import { pct, shareOf } from "./usage-view";
import {
  SPENT_BY,
  SPENT_BY_LABEL,
  SPENT_BY_MEANS,
  SPENT_ON,
  SPENT_ON_LABEL,
  SPENT_ON_MEANS,
  unitsOf,
} from "./usage-spent";
import type { SpentJob, SpentSplit, UsageSpent } from "./usage-spent";

/**
 * What the tokens were spent on, laid out as the other views are: the Jobs that spent most on the left, and in the centre
 * the window's whole split two ways, by what it went on and by who spent it. Each part is a share, one bar a row, all to one
 * scale and most first. Picking a Job puts its own split in the window's place.
 *
 * Keys: `↑↓ j k` move down the Jobs, `↵ o` picks the one under the cursor, `Esc` clears the pick.
 */

type Part = {
  key: string;
  label: string;
  means: string;
  units: number;
};

function onParts(split: SpentSplit): Part[] {
  return SPENT_ON.map((one) => ({
    key: one,
    label: SPENT_ON_LABEL[one],
    means: SPENT_ON_MEANS[one],
    units: split.on[one],
  })).sort((a, b) => b.units - a.units);
}

function byParts(split: SpentSplit): Part[] {
  return SPENT_BY.map((one) => ({
    key: one,
    label: SPENT_BY_LABEL[one],
    means: SPENT_BY_MEANS[one],
    units: split.by[one],
  })).sort((a, b) => b.units - a.units);
}

/** A part a row: its name, what it means on hover, its share as a bar against the largest, and the share itself. */
function PartRows({ label, parts }: { label: string; parts: Part[] }) {
  const whole = parts.reduce((sum, one) => sum + one.units, 0);
  const top = Math.max(0, ...parts.map((one) => one.units));
  return (
    <ul className="armada-usage__shares armada-spent__parts" aria-label={label}>
      {parts.map((one) => (
        <li key={one.key} aria-label={`${one.label}, ${pct(shareOf(one.units, whole))}`}>
          <Tooltip label={one.means}>
            <span className="armada-usage__name">{one.label}</span>
          </Tooltip>
          <span className="armada-usage__track" aria-hidden>
            <span
              className="armada-usage__fill"
              style={{
                width: `${top > 0 ? (one.units / top) * 100 : 0}%`,
                minWidth: "var(--space-1)",
              }}
            />
          </span>
          <Fig tip="Share of the burn">{pct(shareOf(one.units, whole))}</Fig>
        </li>
      ))}
    </ul>
  );
}

function JobRow({
  one,
  whole,
  top,
  cursor,
  picked,
  onPick,
}: {
  one: SpentJob;
  whole: number;
  top: number;
  cursor: boolean;
  picked: boolean;
  onPick: () => void;
}) {
  const units = unitsOf(one.split);
  const most = onParts(one.split)[0]!;
  return (
    <li
      role="option"
      id={`spent-${one.id}`}
      aria-selected={picked}
      aria-label={`${one.title}, ${pct(shareOf(units, whole))}, most on ${most.label}`}
      data-cursor={cursor || undefined}
      className="armada-time__job armada-spent__job"
      onClick={onPick}
    >
      <Tooltip label={one.title}>
        <span className="armada-usage__title">{one.title}</span>
      </Tooltip>
      <span className="armada-usage__track" aria-hidden>
        <span
          className="armada-usage__fill"
          style={{
            width: `${top > 0 ? (units / top) * 100 : 0}%`,
          }}
        />
      </span>
      <Fig tip="Share of the window's burn">{pct(shareOf(units, whole))}</Fig>
      <Tooltip label={most.means}>
        <span className="armada-usage__model">{most.label}</span>
      </Tooltip>
    </li>
  );
}

export function SpentView({ spent }: { spent: UsageSpent }) {
  const [windowId, setWindowId] = useState(spent.windows[spent.windows.length - 1]?.id ?? "");
  const [cursor, setCursor] = useState(-1);
  const [pick, setPick] = useState(-1);
  const shown = spent.windows.find((one) => one.id === windowId) ?? spent.windows[0];
  const jobs = shown?.jobs ?? [];
  const picked = jobs[pick];
  const latest = useRef({
    cursor,
    pick,
    count: jobs.length,
  });
  latest.current = {
    cursor,
    pick,
    count: jobs.length,
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
      const now = latest.current;
      if (event.key === "ArrowDown" || event.key === "j")
        setCursor((at) => Math.min(now.count - 1, at + 1));
      else if (event.key === "ArrowUp" || event.key === "k") setCursor((at) => Math.max(0, at - 1));
      else if ((event.key === "Enter" || event.key === "o") && !onControl) {
        if (now.cursor < 0) return;
        setPick(now.cursor);
      } else if (event.key === "Escape" && now.pick >= 0) setPick(-1);
      else return;
      event.preventDefault();
    }
    window.addEventListener("keydown", pressed);
    return () => window.removeEventListener("keydown", pressed);
  }, []);

  // A Job's place in the list is the window's own, so a new window starts with nothing picked.
  useEffect(() => {
    setCursor(-1);
    setPick(-1);
  }, [
    windowId,
  ]);

  if (shown === undefined) return null;
  const whole = unitsOf(shown.total);
  const top = Math.max(0, ...jobs.map((one) => unitsOf(one.split)));
  const active = jobs[cursor];
  const split = picked?.split ?? shown.total;
  const title = picked?.title ?? "All Jobs";

  return (
    <div className="armada-time armada-spent">
      <div className="armada-time__left">
        <section className="armada-usage__panel" aria-label="Jobs by burn">
          <Band
            keys={[
              "↑↓",
              "↵",
              "Esc",
            ]}
          >
            Jobs
          </Band>
          <div className="armada-time__jobheads armada-spent__job" aria-hidden>
            <span />
            <span />
            <span>Share</span>
            <span>Most on</span>
          </div>
          <ul
            className="armada-time__jobs"
            role="listbox"
            aria-label="Jobs by burn"
            aria-activedescendant={active === undefined ? undefined : `spent-${active.id}`}
            tabIndex={0}
          >
            {jobs.map((one, at) => (
              <JobRow
                key={one.id}
                one={one}
                whole={whole}
                top={top}
                cursor={at === cursor}
                picked={at === pick}
                onPick={() => {
                  setCursor(at);
                  setPick(at);
                }}
              />
            ))}
          </ul>
        </section>
      </div>

      <div className="armada-time__centre">
        <section className="armada-usage__panel" aria-label="Spent on">
          <Band>Spent on</Band>
          <div className="armada-usage__tabs">
            <Tabs
              items={spent.windows.map((one) => ({
                id: one.id,
                label: one.label,
              }))}
              value={shown.id}
              onChange={setWindowId}
            />
          </div>
          <Scope title={title} unit={shown.label} />
          <PartRows label={`${title}, spent on`} parts={onParts(split)} />
        </section>

        <section className="armada-usage__panel" aria-label="Spent by">
          <Band>Spent by</Band>
          <Scope title={title} unit={shown.label} />
          <PartRows label={`${title}, spent by`} parts={byParts(split)} />
        </section>
      </div>
    </div>
  );
}
