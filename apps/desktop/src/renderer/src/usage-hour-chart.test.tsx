// The Usage page's hour chart, drawn by Recharts at a width the browser measures: an hour with no figure breaks the line, builds'
// flags stack in lanes and do not sit on each other, and the readout under the pointer or the keys stays inside the chart.
// What a browser test cannot show is how it looks (colour, line weight, text over a line), and the walk is where that is read.

import { HourChart } from "@armada/components/src/compositions/UsagePage/HourChart";
import type { HourPlot } from "@armada/components/src/compositions/UsagePage/HourChart";
import type { Build } from "@armada/components/src/compositions/UsagePage/usage-builds";
import { flushSync } from "react-dom";
import { createRoot, type Root } from "react-dom/client";
import { afterEach, expect, test } from "vitest";

import "@armada/components/src/look";
import "./styles/index.css";
import { BUILDS, HOURS, hoursIn } from "./usage-time-fixtures";

const roots: {
  root: Root;
  host: HTMLElement;
}[] = [];
afterEach(() => {
  for (const { root, host } of roots.splice(0)) {
    root.unmount();
    host.remove();
  }
});

/** Hours 0 to 9 and 20 to 29 hold a figure, and so does hour 40 alone. */
const values = HOURS.map((_, at) => (at < 10 || (at >= 20 && at < 30) || at === 40 ? 10 : undefined));

const plot: HourPlot = {
  key: "checks",
  figure: "checks",
  title: "Checks per run",
  unit: "min",
  height: 96,
  say: (value) => `${value} min`,
  series: [
    {
      key: "checks",
      name: "Checks per run",
      time: "checks",
      values,
    },
  ],
};

type Drawn = {
  builds?: readonly Build[];
  hour?: number | undefined;
  pick?: string | undefined;
};

function draw(host: HTMLElement, root: Root, drawn: Drawn) {
  flushSync(() =>
    root.render(
      <div style={{ width: "900px" }}>
        <HourChart
          hours={HOURS}
          plots={[plot]}
          label="Median per hour"
          builds={drawn.builds ?? BUILDS}
          pick={drawn.pick}
          onPick={() => {}}
          hour={drawn.hour}
          onHover={() => {}}
          onSelect={() => {}}
        />
      </div>,
    ),
  );
  return host;
}

function mount(drawn: Drawn = {}) {
  const host = document.body.appendChild(document.createElement("div"));
  const root = createRoot(host);
  roots.push({
    root,
    host,
  });
  draw(host, root, drawn);
  return {
    host,
    again: (next: Drawn) => draw(host, root, next),
  };
}

const chart = (host: HTMLElement) => host.querySelector<HTMLElement>('[role="group"]')!;
const lineOf = (host: HTMLElement) =>
  host.querySelector<SVGPathElement>("path.recharts-line-curve")?.getAttribute("d") ?? null;

test("an hour with no figure breaks the line there, and never falls to zero", async () => {
  const { host } = mount();
  await expect.poll(() => lineOf(host)).not.toBeNull();
  const path = lineOf(host)!;
  // Two runs of hours and the hour on its own: three strokes, with nothing drawn across the gaps between them.
  expect(path.match(/M/g)).toHaveLength(3);
  const y = [
    ...path.matchAll(/[ML]([\d.]+)[ ,]([\d.]+)/g),
  ].map((one) => Number(one[2]));
  // Every drawn point is at the one value there is, so none dips to the foot of the plot.
  expect(new Set(y.map((one) => Math.round(one))).size).toBe(1);
  // An hour on its own has no line to join, so it is drawn as a dot.
  expect(host.querySelectorAll("circle.armada-hours__dot")).toHaveLength(1);
});

/**
 * Five builds: three close together that each moved Checks, then two well apart. The fourth sits further from the third than
 * a flag is wide in any font, so the test reads lanes and not the width of a font that may still be loading.
 */
function crowd(): Build[] {
  const base = BUILDS[0]!;
  const at = (id: string, offset: number, checks: number): Build => ({
    ...base,
    id,
    commit: id,
    started: hoursIn(offset),
    branches: [],
    turns: 100,
    checks,
    jobs: [],
  });
  return [
    at("dddddd4", 30, 700),
    at("cccccc3", 21, 500),
    at("bbbbbb2", 20.5, 700),
    at("aaaaaa1", 20, 500),
    at("0000000", 5, 700),
  ];
}

test("flags that would sit on each other take different lanes, and none in a lane overlaps another", async () => {
  const { host } = mount({
    builds: crowd(),
  });
  await expect.poll(() => host.querySelectorAll(".armada-hours__flag")).toHaveLength(5);
  const flags = [
    ...host.querySelectorAll<HTMLElement>(".armada-hours__flagpos"),
  ].map((one) => ({
    lane: one.style.top,
    box: one.querySelector(".armada-hours__flag")!.getBoundingClientRect(),
  }));
  // Three flags start within an hour: they need three lanes between them.
  expect(new Set(flags.map((one) => one.lane)).size).toBeGreaterThanOrEqual(3);
  for (const lane of new Set(flags.map((one) => one.lane))) {
    const among = flags.filter((one) => one.lane === lane).sort((a, b) => a.box.left - b.box.left);
    for (let at = 1; at < among.length; at++)
      expect(among[at]!.box.left).toBeGreaterThanOrEqual(among[at - 1]!.box.right - 0.5);
  }
});

test("the readout stands inside the chart at the first hour, the last, and the middle", async () => {
  const { host, again } = mount({
    hour: 0,
  });
  await expect.poll(() => lineOf(host)).not.toBeNull();
  const inside = (hour: number) => {
    again({
      hour,
    });
    const tip = host.querySelector<HTMLElement>('[role="status"]')!;
    const box = tip.getBoundingClientRect();
    const frame = chart(host).getBoundingClientRect();
    return {
      left: box.left - frame.left,
      right: frame.right - box.right,
    };
  };
  for (const hour of [
    0,
    24,
    47,
  ]) {
    const room = inside(hour);
    expect(room.left, `hour ${hour} past the left edge`).toBeGreaterThanOrEqual(-0.5);
    expect(room.right, `hour ${hour} past the right edge`).toBeGreaterThanOrEqual(-0.5);
  }
});

test("the readout names an hour with no figure and lists no line for it, and a dot stands only where there is a figure", async () => {
  const { host, again } = mount({
    hour: 15,
  });
  const status = () => host.querySelector('[role="status"]')!;
  expect(status().textContent).toContain("Nov 2, 15:00 to 16:00");
  expect(status().textContent).not.toContain("Checks per run");
  expect(host.querySelector("circle.armada-hours__point")).toBeNull();
  again({
    hour: 5,
  });
  expect(status().textContent).toContain("Checks per run10 min");
  await expect.poll(() => host.querySelector("circle.armada-hours__point")).not.toBeNull();
});
