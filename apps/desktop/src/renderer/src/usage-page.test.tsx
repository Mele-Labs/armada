// The Usage page over a summary: what it leaves empty, how a Job without a title reads, what the keys do, and the labels the
// cuts carry. It draws what it is handed, so the page is mounted bare, with the mock's summary.

import { UsagePage } from "@armada/components";
import type { UsageSummary } from "@armada/protocol";
import { flushSync } from "react-dom";
import { createRoot, type Root } from "react-dom/client";
import { afterEach, expect, test } from "vitest";

import { JOB_TITLES, usageSummaryNow } from "./mock/usage-summary";

const roots: { root: Root; host: HTMLElement }[] = [];
afterEach(() => {
  for (const { root, host } of roots.splice(0)) {
    root.unmount();
    host.remove();
  }
});

function mount(props: { summary?: UsageSummary; titles?: ReadonlyMap<string, string>; planPercent?: number }): HTMLElement {
  const host = document.body.appendChild(document.createElement("div"));
  const root = createRoot(host);
  roots.push({ root, host });
  flushSync(() => root.render(<UsagePage {...props} />));
  return host;
}

/** A key as the keyboard sends one, and the render it causes let through before the next, as a person's gap between keys does. */
const press = async (key: string) => {
  document.body.dispatchEvent(new KeyboardEvent("keydown", { key, bubbles: true, cancelable: true }));
  await new Promise((done) => setTimeout(done, 30));
};
const text = (host: HTMLElement, selector: string) => host.querySelector(selector)?.textContent ?? null;
const summary = (): UsageSummary => usageSummaryNow(0);

test("before Fleet has answered, every panel is drawn and none holds a figure", () => {
  const host = mount({});
  expect([...host.querySelectorAll(".armada-usage__band")].map((one) => one.firstElementChild?.textContent)).toEqual(["Jobs", "Breakdown", "Burn per day", "Day and hour"]);
  expect(host.querySelectorAll('[role="option"]')).toHaveLength(0);
  expect(host.querySelector(".armada-usage__tiles")).toBeNull();
  expect(host.querySelector("svg")).toBeNull();
  expect(host.querySelector('[role="grid"]')).toBeNull();
});

test("the plan tile is left out where the plan has no reading, and drawn where it has", () => {
  expect(mount({ summary: summary() }).querySelector('[aria-label="Weekly plan used"]')).toBeNull();
  const host = mount({ summary: summary(), planPercent: 66 });
  expect(text(host, '[aria-label="Weekly plan used"] .armada-usage__big .armada-usage__fig')).toBe("66%");
});

test("a Job the page holds no title for shows its short id, and the rest is one quiet row", () => {
  const withRest: UsageSummary = { ...summary(), rest: { sessions: 9, usage: summary().total } };
  const titles = new Map(JOB_TITLES);
  const first = withRest.sessions[0]!.session_id;
  titles.delete(first);
  const host = mount({ summary: withRest, titles });
  const options = [...host.querySelectorAll('[role="option"]')];
  expect(options).toHaveLength(withRest.sessions.length);
  expect(options[0]!.querySelector(".armada-usage__title")?.textContent).toBe(first.slice(-6));
  expect(options[0]!.querySelector(".armada-usage__title")?.hasAttribute("data-id")).toBe(true);
  expect(options[1]!.querySelector(".armada-usage__title")?.hasAttribute("data-id")).toBe(false);
  expect(host.querySelector(".armada-usage__rest")?.firstElementChild?.textContent).toBe("rest");
});

test("the tile deltas come from the prior period, and are absent where it held nothing", () => {
  const withPrior = mount({ summary: summary(), planPercent: 66 });
  expect(withPrior.querySelectorAll('[aria-label="Burn in subagents"] .armada-usage__delta')).toHaveLength(1);
  const none = mount({ summary: { ...summary(), prior: { ...summary().prior, units: 0 } }, planPercent: 66 });
  expect(none.querySelector('[aria-label="Burn in subagents"] .armada-usage__delta')).toBeNull();
  expect(none.querySelector('[aria-label="Above 200k context"] .armada-usage__delta')).toBeNull();
});

test("the heat map's columns are weekdays, Monday first", () => {
  const host = mount({ summary: summary() });
  expect([...host.querySelectorAll('[role="columnheader"]')].map((one) => one.textContent).filter(Boolean)).toEqual(["Mon", "Tue", "Wed", "Thu", "Fri", "Sat", "Sun"]);
});

test("↓ ↵ picks the top Job and Esc returns to the week", async () => {
  const host = mount({ summary: summary(), titles: JOB_TITLES, planPercent: 66 });
  const picked = () => host.querySelector('[role="option"][aria-selected="true"] .armada-usage__title')?.textContent ?? null;
  const planFigure = () => text(host, '[aria-label="Weekly plan used"] .armada-usage__big .armada-usage__fig');
  await press("ArrowDown");
  await press("Enter");
  await expect.poll(picked).toBe("Jobs -> Session Merger");
  expect(planFigure()).not.toBe("66%");
  await press("Escape");
  await expect.poll(picked).toBeNull();
  expect(planFigure()).toBe("66%");
});

test("m t b y d set the breakdown, with no tool, GitHub and the token kinds named", async () => {
  const host = mount({ summary: summary() });
  const region = () => host.querySelector('[role="region"][aria-label^="By "]')?.getAttribute("aria-label");
  const names = () => [...host.querySelectorAll('[role="region"][aria-label^="By "] li')].map((one) => one.getAttribute("aria-label")!.split(",")[0]);
  expect(region()).toBe("By model");
  await press("t");
  await expect.poll(region).toBe("By tool");
  expect(names()).toContain("no tool");
  expect(names()).not.toContain("none");
  await press("b");
  await expect.poll(region).toBe("By Bash use");
  expect(names()).toContain("GitHub");
  expect(names()).toHaveLength(7);
  await press("y");
  await expect.poll(region).toBe("By token type");
  expect(names()).toEqual(["cache read", "cache creation", "output", "input"]);
  await press("d");
  await expect.poll(region).toBe("By day");
  expect(names()).toHaveLength(7);
  await press("m");
  await expect.poll(region).toBe("By model");
});

test("the first day key lands on the busiest cell, the next steps on, and Esc clears the cell before the pick", async () => {
  const host = mount({ summary: summary(), titles: JOB_TITLES });
  const cell = () => host.querySelector('[role="gridcell"][aria-selected="true"]')?.getAttribute("aria-label") ?? null;
  await press("ArrowDown");
  await press("Enter");
  await press("l");
  await expect.poll(cell).toMatch(/^Wed, 21h to 00h/);
  await press("l");
  await expect.poll(cell).toMatch(/^Thu, 21h to 00h/);
  await press("[");
  await expect.poll(cell).toMatch(/^Thu, 18h to 21h/);
  await press("Escape");
  await expect.poll(cell).toBeNull();
  expect(host.querySelector('[role="option"][aria-selected="true"]')).not.toBeNull();
  await press("Escape");
  await expect.poll(() => host.querySelector('[role="option"][aria-selected="true"]')).toBeNull();
});
