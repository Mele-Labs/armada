// The Usage page's Time view as it was approved by walking it: the switch from Tokens, the shape it draws while pending, the
// Jobs and their split bars, the keys, the split sheet and what is marked far over, and what picking a build narrows. Mounted
// bare over the props the page takes; no wire carries any of it yet. Real keys, in a real browser, since the page listens on
// the window and a dispatched event would skip the focus a press moves.

import { UsagePage } from "@armada/components";
import type { UsagePageProps } from "@armada/components/src/compositions/UsagePage/UsagePage";
import { flushSync } from "react-dom";
import { createRoot, type Root } from "react-dom/client";
import { afterEach, expect, test } from "vitest";
import { userEvent } from "vitest/browser";

import "@armada/components/src/look";
import "./styles/index.css";
import { BUILDS, HOURS, ODD, SLOW, TIME } from "./usage-time-fixtures";

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

function mount(props: UsagePageProps): HTMLElement {
  const host = document.body.appendChild(document.createElement("div"));
  const root = createRoot(host);
  roots.push({
    root,
    host,
  });
  flushSync(() => root.render(<UsagePage {...props} />));
  return host;
}

/** The whole Time view, drawn. */
const timed = (): UsagePageProps => ({
  time: TIME,
  builds: BUILDS,
  hours: HOURS,
});

const keys = (...pressed: string[]) => userEvent.keyboard(pressed.join(""));

/** What a tooltip says about an element: the text of what its `aria-describedby` names. */
const described = (el: Element | null | undefined): string =>
  (el?.getAttribute("aria-describedby") ?? "")
    .split(" ")
    .map((id) => document.getElementById(id)?.textContent ?? "")
    .join(" ")
    .trim();

const tab = (host: HTMLElement, name: string) =>
  [
    ...host.querySelectorAll('[role="tab"]'),
  ].find((one) => one.textContent === name);
const jobs = (host: HTMLElement) =>
  host.querySelector<HTMLElement>('[role="listbox"][aria-label="Slowest Jobs"]');
const options = (host: HTMLElement) => [
  ...(jobs(host)?.querySelectorAll('[role="option"]') ?? []),
];
const active = (host: HTMLElement) => jobs(host)?.getAttribute("aria-activedescendant") ?? null;
const dialog = () => document.querySelector<HTMLElement>('[role="dialog"]');
const scope = (host: HTMLElement) =>
  host.querySelector('section[aria-label="Split"] .armada-usage__title')?.textContent ?? null;
const readout = (host: HTMLElement) =>
  host.querySelector('section[aria-label="Hour by hour"] [role="status"]')?.firstElementChild
    ?.textContent ?? null;
const splitRows = (root: ParentNode, label: string) => [
  ...root.querySelectorAll(`ul[aria-label="${label}"] > li`),
];
const pressBuild = (host: HTMLElement, id: string) =>
  host.querySelector<HTMLElement>(`#build-${id}`)!.click();

test("v switches between Tokens and Time", async () => {
  const host = mount(timed());
  expect(tab(host, "Tokens")?.getAttribute("aria-selected")).toBe("true");
  expect(jobs(host)).toBeNull();
  await keys("v");
  await expect.poll(() => tab(host, "Time")?.getAttribute("aria-selected")).toBe("true");
  expect(jobs(host)).not.toBeNull();
  await keys("v");
  await expect.poll(() => tab(host, "Tokens")?.getAttribute("aria-selected")).toBe("true");
  expect(jobs(host)).toBeNull();
});

test("a page given no time offers no Time view, and v does nothing", async () => {
  const host = mount({});
  expect(tab(host, "Time")).toBeUndefined();
  await keys("v");
  expect(host.querySelector(".armada-time")).toBeNull();
  expect(tab(host, "Time")).toBeUndefined();
});

test("a pending Time view draws skeleton shapes and no figure", async () => {
  const host = mount({
    time: "pending",
    builds: "pending",
    hours: "pending",
  });
  await keys("v");
  await expect.poll(() => host.querySelector(".armada-time")).not.toBeNull();
  expect(host.querySelector(".armada-time")?.getAttribute("aria-busy")).toBe("true");
  expect(host.querySelectorAll(".armada-skeleton").length).toBeGreaterThan(0);
  expect(host.querySelector(".armada-usage__fig")).toBeNull();
  expect(host.querySelector('[role="option"]')).toBeNull();
  expect(host.querySelector("svg.recharts-surface")).toBeNull();
});

test("a Job's split bar has a segment for each bucket with time, and a tooltip naming its span and share", async () => {
  const host = mount(timed());
  await keys("v");
  await expect.poll(() => options(host)).toHaveLength(3);
  const parts = (row: Element) => [
    ...row.querySelectorAll<HTMLElement>(".armada-time__track [data-time]"),
  ];
  const slow = parts(options(host)[0]!);
  // Stopped and Idle hold nothing in this Job, so neither draws.
  expect(slow.map((one) => one.dataset.time)).toEqual([
    "model",
    "tools",
    "checks",
    "subagents",
    "slot",
    "you",
    "pull_request",
  ]);
  expect(described(slow[0])).toBe("Model 1h 19m, 19%");
  expect(described(slow[2])).toBe("Checks 2h 28m, 35%");
  expect(parts(options(host)[2]!).map((one) => one.dataset.time)).toEqual([
    "model",
    "tools",
    "checks",
  ]);
});

test("↑↓ and j k move the cursor down the Jobs, and stop at either end", async () => {
  const host = mount(timed());
  await keys("v");
  await expect.poll(() => options(host)).toHaveLength(3);
  expect(active(host)).toBeNull();
  await keys("j");
  await expect.poll(() => active(host)).toBe("time-job-slow");
  await keys("{ArrowDown}");
  await expect.poll(() => active(host)).toBe("time-job-fix");
  await keys("j", "j", "j");
  await expect.poll(() => active(host)).toBe("time-job-odd");
  await keys("k");
  await expect.poll(() => active(host)).toBe("time-job-fix");
  await keys("{ArrowUp}", "{ArrowUp}", "k");
  await expect.poll(() => active(host)).toBe("time-job-slow");
});

test("↵ and o open the Job under the cursor in a sheet, and Esc closes it", async () => {
  const host = mount(timed());
  await keys("v");
  await expect.poll(() => options(host)).toHaveLength(3);
  await keys("{Enter}");
  expect(dialog()).toBeNull();
  await keys("j", "{Enter}");
  await expect.poll(() => dialog()?.getAttribute("aria-label")).toBe(SLOW.title);
  expect(splitRows(dialog()!, "Job split").length).toBeGreaterThan(0);
  await keys("{Escape}");
  await expect.poll(() => dialog()).toBeNull();
  await keys("j", "o");
  await expect.poll(() => dialog()?.getAttribute("aria-label")).toBe("Fix the stale label");
  await keys("{Escape}");
  await expect.poll(() => dialog()).toBeNull();
});

test("a bucket far over its Workflow's usual is marked with the usual beside it, and one just under either line is not", async () => {
  const host = mount(timed());
  await keys("v");
  await expect.poll(() => options(host)).toHaveLength(3);
  await keys("j", "{Enter}");
  await expect.poll(() => dialog()).not.toBeNull();
  const rows = splitRows(dialog()!, "Job split");
  const row = (name: string) => rows.find((one) => one.getAttribute("aria-label")?.startsWith(name))!;
  // Checks 148m against a usual of 12m, and Pull request 121m against 12m.
  expect(row("Checks").hasAttribute("data-over")).toBe(true);
  expect(row("Checks").getAttribute("aria-label")).toBe("Checks, 2 hours 28 minutes, usual 12 minutes");
  expect(row("Pull request").hasAttribute("data-over")).toBe(true);
  // Model 79m against 40m is just under twice; Slot 13m against 4m is over twice but 9 minutes past.
  for (const name of [
    "Model",
    "Slot",
    "Tools",
  ]) {
    expect(row(name).hasAttribute("data-over")).toBe(false);
    expect(row(name).getAttribute("aria-label")).not.toContain("usual");
  }
  // The usual itself is drawn in the row, named for the Workflow it is the median of.
  const usual = [
    ...row("Checks").querySelectorAll(".armada-usage__fig"),
  ].at(-1);
  expect(usual?.textContent).toBe("12m");
  expect(described(usual?.closest("[aria-describedby]"))).toBe("Usual for Build a feature: the median Job, 12 minutes");
  expect(
    [
      ...dialog()!.querySelectorAll(".armada-time__heads span"),
    ].map((one) => one.textContent),
  ).toContain("Usual");
});

test("a Job whose Workflow has no usual is marked nowhere, and its sheet reads each bucket as a share", async () => {
  const host = mount(timed());
  await keys("v");
  await expect.poll(() => options(host)).toHaveLength(3);
  await keys("j", "j", "j", "{Enter}");
  await expect.poll(() => dialog()?.getAttribute("aria-label")).toBe(ODD.title);
  const rows = splitRows(dialog()!, "Job split");
  expect(rows.length).toBe(3);
  expect(rows.some((one) => one.hasAttribute("data-over"))).toBe(false);
});

test("picking a build narrows the Jobs and the split to it, and Esc clears it", async () => {
  const host = mount(timed());
  await keys("v");
  await expect.poll(() => options(host)).toHaveLength(3);
  expect(scope(host)).toBe("All Jobs");
  pressBuild(host, "7ac3d52");
  await expect.poll(() => options(host).length).toBe(2);
  expect(options(host).map((one) => one.id)).toEqual([
    "time-job-slow",
    "time-job-fix",
  ]);
  expect(scope(host)).toBe("Build 7ac3d52");
  expect(splitRows(host, "Split").map((one) => one.getAttribute("aria-label"))).toEqual([
    "Checks, 1 hour 30 minutes",
    "Model, 30 minutes",
  ]);
  await keys("{Escape}");
  await expect.poll(() => options(host).length).toBe(3);
  expect(scope(host)).toBe("All Jobs");
});

test("← → and h l step the hour readout, and do not narrow anything", async () => {
  const host = mount(timed());
  await keys("v");
  await expect.poll(() => options(host)).toHaveLength(3);
  expect(readout(host)).toBeNull();
  await keys("l");
  // The first press lands on the latest hour with burn in it: hour 44, 20:00 on Nov 3.
  await expect.poll(() => readout(host)).toBe("Nov 3, 20:00 to 21:00");
  await keys("{ArrowRight}");
  await expect.poll(() => readout(host)).toBe("Nov 3, 21:00 to 22:00");
  await keys("h");
  await expect.poll(() => readout(host)).toBe("Nov 3, 20:00 to 21:00");
  await keys("{ArrowLeft}");
  await expect.poll(() => readout(host)).toBe("Nov 3, 19:00 to 20:00");
  expect(options(host)).toHaveLength(3);
  expect(scope(host)).toBe("All Jobs");
});

test("Esc clears the hour, and then the build", async () => {
  const host = mount(timed());
  await keys("v");
  await expect.poll(() => options(host)).toHaveLength(3);
  pressBuild(host, "7ac3d52");
  await expect.poll(() => scope(host)).toBe("Build 7ac3d52");
  await keys("l");
  await expect.poll(() => readout(host)).not.toBeNull();
  await keys("{Escape}");
  await expect.poll(() => readout(host)).toBeNull();
  expect(scope(host)).toBe("Build 7ac3d52");
  await keys("{Escape}");
  await expect.poll(() => scope(host)).toBe("All Jobs");
});

test("Esc closes the Job's sheet before the build", async () => {
  const host = mount(timed());
  await keys("v");
  await expect.poll(() => options(host)).toHaveLength(3);
  pressBuild(host, "7ac3d52");
  await expect.poll(() => scope(host)).toBe("Build 7ac3d52");
  await keys("j", "{Enter}");
  await expect.poll(() => dialog()).not.toBeNull();
  await keys("{Escape}");
  await expect.poll(() => dialog()).toBeNull();
  expect(scope(host)).toBe("Build 7ac3d52");
  await keys("{Escape}");
  await expect.poll(() => scope(host)).toBe("All Jobs");
});
