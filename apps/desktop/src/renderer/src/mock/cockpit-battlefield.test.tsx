// The flight deck's battlefield through `App` on the ship scenario: a drone for each Session, a marker on a drone that holds a call
// and an edge marker for one out of the pane, a dot of Activity that focuses its drone and plays on to Live, and no two hit boxes on the
// field meeting. The canvas draws the pictures; these read the layer of elements over it that can be pressed, hovered and tabbed to.
// Hand-run with the rest of `mock/`. #2301.

import { expect, test } from "vitest";
import { page, userEvent } from "vitest/browser";

import { mount, onScreen, unmountAfterEach } from "./testing";

unmountAfterEach();

const list = () => page.getByRole("region", { name: "Jobs", exact: true });
const controls = () => page.getByRole("region", { name: "Flight controls" });
const rows = () => [...document.querySelectorAll(".deck-list .deck-row")].map((one) => one.getAttribute("aria-label") ?? "");
const drones = () => [...document.querySelectorAll<HTMLElement>(".bf [data-hit=drone]")];
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

test("the battlefield has one drone for each Session", async () => {
  await map();
  await expect.poll(() => drones().length).toBeGreaterThan(0);
  const keyed = drones().map((one) => one.getAttribute("data-label") ?? "");
  // One drone for every Session of the list, named as the list names it.
  for (const row of rows()) expect(keyed).toContain(row);
  expect(drones()).toHaveLength(rows().length);
});

test("a drone that holds a call carries a call marker, and an off-screen one an edge marker that eases the camera to it", async () => {
  await map();
  const marked = () => [...document.querySelectorAll<HTMLElement>("[data-hit=call]")].map((one) => one.getAttribute("aria-label") ?? "");
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

  // The marker hangs on the border and bobs a little, so it is pressed as a press, not waited on for stillness.
  edge.click();
  // The camera eases to it: its own marker is now in the pane.
  await expect.poll(() => [...document.querySelectorAll<HTMLElement>("[data-hit=call]")].some((one) => one.getAttribute("aria-label") === title && inside(one)), { timeout: 4000 }).toBe(true);
});

test("pressing an Activity dot focuses its drone and Replay from here plays forward to Live and clears the focus", async () => {
  await map();
  await expect.poll(() => document.querySelectorAll(".deck-activity .armada-tl__dot").length).toBeGreaterThan(0);
  // The newest event is the last dot: they are drawn oldest first.
  [...document.querySelectorAll<HTMLElement>(".deck-activity .armada-tl__dot")].at(-1)!.click();

  // The drone the event is about is focused, the rest dimmed, and a line says what moved.
  await expect.element(page.getByRole("region", { name: "Event" })).toBeVisible();
  expect(document.querySelectorAll(".bf [data-hit=drone][data-focus]")).toHaveLength(1);
  expect(document.querySelectorAll(".bf [data-hit=drone][data-dim]").length).toBeGreaterThan(0);

  await page.getByRole("button", { name: "Replay from here" }).click();
  await expect.element(page.getByRole("region", { name: "Replay" })).toBeVisible();
  expect(document.querySelector(".deck-replay-mark")).not.toBeNull();
  await page.getByRole("button", { name: "16x" }).click();
  // It plays forward to now, and arriving at now is Live: the bar, the marker and the focus are gone.
  await expect.poll(() => document.querySelector(".deck-replay") === null, { timeout: 40_000 }).toBe(true);
  expect(document.querySelector(".deck-replay-mark")).toBeNull();
  expect(document.querySelector('[role="region"][aria-label="Event"]')).toBeNull();
  expect(document.querySelectorAll(".bf [data-hit=drone][data-focus]")).toHaveLength(0);
});

test("Esc leaves a replay for Live, and Space plays and pauses it", async () => {
  await map();
  await expect.poll(() => document.querySelectorAll(".deck-activity .armada-tl__dot").length).toBeGreaterThan(0);
  [...document.querySelectorAll<HTMLElement>(".deck-activity .armada-tl__dot")].at(-1)!.click();
  await page.getByRole("button", { name: "Replay from here" }).click();
  const bar = page.getByRole("region", { name: "Replay" });
  await expect.element(bar).toBeVisible();
  field()!.focus();
  await userEvent.keyboard(" ");
  await expect.element(bar.getByRole("button", { name: "Play" })).toBeVisible();
  await userEvent.keyboard("{Escape}");
  await expect.poll(() => document.querySelector(".deck-replay") === null).toBe(true);
});

test("every interactive hit box on the field is clear of another at rest", async () => {
  await map();
  await expect.poll(() => drones().length).toBeGreaterThan(0);
  const hits = [...document.querySelectorAll<HTMLElement>(".bf :is([data-hit=drone], [data-hit=pad], [data-hit=goal], [data-hit=outpost], [data-hit=payload], [data-hit=call], [data-hit=clash]),.bf .bf-zoom button")].map((one) => ({ one, at: box(one) })).filter((one) => one.at.width > 0 && one.at.height > 0);
  expect(hits.length).toBeGreaterThan(8);
  const clashing: string[] = [];
  for (let i = 0; i < hits.length; i += 1) {
    for (let j = i + 1; j < hits.length; j += 1) {
      // A drone's badge hangs from its own drone, and is the one thing that may stand over it.
      const [a, b] = [hits[i]!.one.getAttribute("data-key"), hits[j]!.one.getAttribute("data-key")];
      if (a !== null && a === b) continue;
      if (meets(hits[i]!.at, hits[j]!.at)) clashing.push(`${label(hits[i]!.one)} meets ${label(hits[j]!.one)}`);
    }
  }
  expect(clashing).toEqual([]);
});

function label(one: Element): string {
  const owner = one.closest("[aria-label]");
  return owner?.getAttribute("aria-label") ?? one.className.toString();
}
