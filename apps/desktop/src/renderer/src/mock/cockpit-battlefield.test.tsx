// The flight deck's battlefield through `App` on the ship scenario: a drone for each Job and Session at rest by its pad where no
// path is known, a marker on a drone that holds a call and an edge marker for one out of the pane, a row of Activity that focuses
// its drone and plays on to Live, a Job's mission and its canvas showing the same run, and no two hit boxes on the field meeting.
// Hand-run with the rest of `mock/`. #2301.

import { expect, test } from "vitest";
import { page, userEvent } from "vitest/browser";

import { mount, onScreen, unmountAfterEach } from "./testing";

unmountAfterEach();

const list = () => page.getByRole("region", { name: "Jobs and Sessions" });
const controls = () => page.getByRole("region", { name: "Flight controls" });
const rows = () => [...document.querySelectorAll(".deck-list .deck-row")].map((one) => one.getAttribute("aria-label") ?? "");
const drones = () => [...document.querySelectorAll<HTMLElement>(".bf-drone")];
const field = () => document.querySelector<HTMLElement>(".bf");

async function map() {
  const app = mount("cockpit-ship-flightdeck");
  await onScreen();
  await expect.element(list()).toBeVisible();
  await controls().getByRole("button", { name: "Map" }).click();
  await expect.poll(() => field() !== null).toBe(true);
  // The camera has eased to rest and the drones have settled.
  await new Promise((done) => window.setTimeout(done, 900));
  return app;
}

const box = (el: Element) => el.getBoundingClientRect();
const meets = (a: DOMRect, b: DOMRect) => a.left < b.right && a.right > b.left && a.top < b.bottom && a.bottom > b.top;

test("the battlefield draws one drone for each Job and Session and rests them by their pads when no path is known", async () => {
  await map();
  await expect.poll(() => drones().length).toBeGreaterThan(0);
  const keyed = drones().map((one) => one.getAttribute("data-label") ?? "");
  // One drone for every Job and Session of the list, named as the list names it.
  for (const row of rows()) expect(keyed).toContain(row);
  expect(drones()).toHaveLength(rows().length);

  // A drone no edit places is over no plot and has no touch line: it stands by its pad or at its base.
  const idle = drones().find((one) => one.getAttribute("data-label") === "Cache key")!;
  expect(idle).toBeDefined();
  expect(document.querySelector(`.bf-touch[data-for="${idle.getAttribute("data-key")}"]`)).toBeNull();
  // And one that edited a path the repository draws is over its plot.
  const edited = drones().find((one) => one.getAttribute("data-label") === "Gate policy on every run")!;
  expect(document.querySelector(`.bf-touch[data-for="${edited.getAttribute("data-key")}"]`)).not.toBeNull();
});

test("a drone that holds a call carries a call marker, and an off-screen one an edge marker that eases the camera to it", async () => {
  await map();
  const marked = () => [...document.querySelectorAll<HTMLElement>(".bf-call")].map((one) => one.getAttribute("aria-label") ?? "");
  await expect.poll(() => marked().length).toBeGreaterThan(0);
  expect(marked()).toContain("Cache key needs you");
  // At rest the whole field is in the pane, so no marker is out of it.
  expect(document.querySelectorAll(".bf-edge")).toHaveLength(0);

  // Close in, away from where the calls are: one is out of the pane and its marker is on the border.
  const zoom = page.getByRole("button", { name: "Zoom in" });
  for (let i = 0; i < 6; i += 1) await zoom.click();
  await expect.poll(() => document.querySelectorAll(".bf-edge").length).toBeGreaterThan(0);
  const edge = document.querySelector<HTMLElement>(".bf-edge")!;
  const title = edge.getAttribute("aria-label")!;
  expect(title).toMatch(/needs you$/);
  const inside = (e: Element) => {
    const pane = box(field()!);
    const b = box(e);
    return b.left >= pane.left && b.right <= pane.right && b.top >= pane.top && b.bottom <= pane.bottom;
  };
  expect(inside(edge)).toBe(true);

  await page.getByRole("button", { name: title }).first().click();
  // The camera eases to it: its own marker is now in the pane.
  await expect.poll(() => [...document.querySelectorAll<HTMLElement>(".bf-call")].some((one) => one.getAttribute("aria-label") === title && inside(one)), { timeout: 4000 }).toBe(true);
});

test("pressing an Activity row focuses its drone and Replay from here plays forward to Live and clears the focus", async () => {
  await map();
  const feed = page.getByRole("region", { name: "Activity" });
  await expect.poll(() => document.querySelectorAll(".deck-activity [data-event]").length).toBeGreaterThan(0);
  const row = document.querySelector<HTMLElement>(".deck-activity [data-event]")!;
  row.click();

  // The drone the event is about is focused, the rest dimmed, and a line says what moved.
  await expect.element(page.getByRole("region", { name: "Event" })).toBeVisible();
  expect(document.querySelectorAll(".bf-drone[data-focus]")).toHaveLength(1);
  expect(document.querySelectorAll(".bf-drone[data-dim]").length).toBeGreaterThan(0);
  void feed;

  await page.getByRole("button", { name: "Replay from here" }).click();
  await expect.element(page.getByRole("region", { name: "Replay" })).toBeVisible();
  expect(document.querySelector(".deck-replay-mark")).not.toBeNull();
  await page.getByRole("button", { name: "16x" }).click();
  // It plays forward to now, and arriving at now is Live: the bar, the marker and the focus are gone.
  await expect.poll(() => document.querySelector(".deck-replay") === null, { timeout: 40_000 }).toBe(true);
  expect(document.querySelector(".deck-replay-mark")).toBeNull();
  expect(document.querySelector('[role="region"][aria-label="Event"]')).toBeNull();
  expect(document.querySelectorAll(".bf-drone[data-focus]")).toHaveLength(0);
});

test("Esc leaves a replay for Live, and Space plays and pauses it", async () => {
  await map();
  await expect.poll(() => document.querySelectorAll(".deck-activity [data-event]").length).toBeGreaterThan(0);
  document.querySelector<HTMLElement>(".deck-activity [data-event]")!.click();
  await page.getByRole("button", { name: "Replay from here" }).click();
  const bar = page.getByRole("region", { name: "Replay" });
  await expect.element(bar).toBeVisible();
  field()!.focus();
  await userEvent.keyboard(" ");
  await expect.element(bar.getByRole("button", { name: "Play" })).toBeVisible();
  await userEvent.keyboard("{Escape}");
  await expect.poll(() => document.querySelector(".deck-replay") === null).toBe(true);
});

test("a Job's Mission and Canvas views show the same acts", async () => {
  mount("cockpit-ship-flightdeck");
  await onScreen();
  await list().getByRole("button", { name: "Shorten the reconnect wait" }).click();
  const pane = () => document.querySelector<HTMLElement>('.deck-pane[aria-label^="Job: Shorten the reconnect wait"]')!;
  await expect.poll(() => pane() !== null).toBe(true);
  expect(pane().querySelector('[role="group"][aria-label="Mission"]')).not.toBeNull();

  // The head's acts are the pane's, and are the same whichever view is up.
  const acts = () => [...pane().querySelectorAll(".deck-pane__acts button")].map((one) => one.getAttribute("aria-label") ?? one.textContent).filter((one) => !/^(Mission|Canvas)$/.test(one ?? ""));
  const missionActs = acts();
  expect(missionActs).toContain("Open the Job page");
  // The mission's pins are the run's steps, the canvas's cards are the same steps.
  await expect.poll(() => pane().querySelectorAll(".bf-pin[data-pin^='step:']").length).toBeGreaterThan(0);
  const pins = [...pane().querySelectorAll(".bf-pin[data-pin^='step:']")].map((one) => one.getAttribute("data-label") ?? "");

  await userEvent.keyboard("v");
  await expect.poll(() => pane().querySelector('[role="group"][aria-label="Canvas"]') !== null).toBe(true);
  expect(acts()).toEqual(missionActs);
  await expect.poll(() => pane().querySelectorAll(".armada-wfstep, [data-step-card]").length).toBeGreaterThan(0);
  const canvas = pane().textContent ?? "";
  for (const name of pins) expect(canvas).toContain(name);

  await userEvent.keyboard("v");
  await expect.poll(() => pane().querySelector('[role="group"][aria-label="Mission"]') !== null).toBe(true);
});

test("every interactive hit box on the field is clear of another at rest", async () => {
  await map();
  await expect.poll(() => drones().length).toBeGreaterThan(0);
  const hits = [...document.querySelectorAll<SVGElement>(".bf .bf-hit, .bf .bf-call, .bf .bf-zoom button")].map((one) => ({ one, at: box(one) })).filter((one) => one.at.width > 0 && one.at.height > 0);
  expect(hits.length).toBeGreaterThan(8);
  const clashing: string[] = [];
  for (let i = 0; i < hits.length; i += 1) {
    for (let j = i + 1; j < hits.length; j += 1) {
      // A drone's badge hangs from its own drone, and is the one thing that may stand over it.
      if (hits[i]!.one.closest(".bf-drone") !== null && hits[i]!.one.closest(".bf-drone") === hits[j]!.one.closest(".bf-drone")) continue;
      if (meets(hits[i]!.at, hits[j]!.at)) clashing.push(`${label(hits[i]!.one)} meets ${label(hits[j]!.one)}`);
    }
  }
  expect(clashing).toEqual([]);
});

function label(one: Element): string {
  const owner = one.closest("[aria-label]");
  return owner?.getAttribute("aria-label") ?? one.className.toString();
}
