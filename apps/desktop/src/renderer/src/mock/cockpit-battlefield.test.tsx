// The flight deck's battlefield through `App` on the ship scenario: a drone for each Session at rest by its pad where no
// path is known, a marker on a drone that holds a call and an edge marker for one out of the pane, a dot of Activity that focuses
// its drone and plays on to Live, and no two hit boxes on the field meeting.
// Hand-run with the rest of `mock/`. #2301.

import { expect, test } from "vitest";
import { page, userEvent } from "vitest/browser";

import type { Session } from "@armada/screens/src/draft/sessions";
import { repository } from "@armada/screens/src/fixtures/build/base";

import { scenarioNamed } from "./scenario";
import { motion, mount, onScreen, unmountAfterEach } from "./testing";

unmountAfterEach();

const list = () => page.getByRole("region", { name: "Work", exact: true });
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

test("the battlefield draws one drone for each Session and rests them by their pads when no path is known", async () => {
  await map();
  await expect.poll(() => drones().length).toBeGreaterThan(0);
  const keyed = drones().map((one) => one.getAttribute("data-label") ?? "");
  // One drone for every Session of the list, named as the list names it.
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

  // The marker hangs on the border and bobs a little, so it is pressed as a press, not waited on for stillness.
  edge.click();
  // The camera eases to it: its own marker is now in the pane.
  await expect.poll(() => [...document.querySelectorAll<HTMLElement>(".bf-call")].some((one) => one.getAttribute("aria-label") === title && inside(one)), { timeout: 4000 }).toBe(true);
});

test("pressing an Activity dot focuses its drone and Replay from here plays forward to Live and clears the focus", async () => {
  await map();
  await expect.poll(() => document.querySelectorAll(".deck-activity .armada-tl__dot").length).toBeGreaterThan(0);
  // The newest event is the last dot: they are drawn oldest first.
  [...document.querySelectorAll<HTMLElement>(".deck-activity .armada-tl__dot")].at(-1)!.click();

  // The drone the event is about is focused, the rest dimmed, and a line says what moved.
  await expect.element(page.getByRole("region", { name: "Event" })).toBeVisible();
  expect(document.querySelectorAll(".bf-drone[data-focus]")).toHaveLength(1);
  expect(document.querySelectorAll(".bf-drone[data-dim]").length).toBeGreaterThan(0);

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

// The loops that say a drone is working or calling are the only ones on the field, and the camera does not drift: every loop is a layer
// the browser draws again each frame, and 59 of them kept the deck at a third of the display's rate.
test("An idle drone runs no animation, a calling one pings, and no rotor turns", async () => {
  await motion();
  await map();
  const loops = (el: Element) =>
    el
      .getAnimations({ subtree: true })
      .filter((one): one is CSSAnimation => one instanceof CSSAnimation && one.playState === "running" && one.effect?.getTiming().iterations === Infinity)
      .map((one) => one.animationName);
  const all = drones();
  const idle = all.filter((one) => !one.hasAttribute("data-hover") && !one.hasAttribute("data-ping"));
  const working = all.filter((one) => one.hasAttribute("data-spin"));
  const calling = all.filter((one) => one.hasAttribute("data-ping"));
  // A resting Session holds a call until it is seen, so the ship has no drone at rest and calm; if one is, it runs no loop.
  expect(working.length).toBeGreaterThan(0);
  expect(calling.length).toBeGreaterThan(0);

  for (const one of idle) expect(loops(one), one.getAttribute("data-label") ?? "").toEqual([]);
  // The rotors stand still for the machine (the owner, 15 Oct 2026).
  for (const one of working) expect(loops(one), one.getAttribute("data-label") ?? "").not.toContain("bf-spin");
  for (const one of calling) expect(loops(one), one.getAttribute("data-label") ?? "").toContain("bf-ping");
  expect(document.getAnimations().map((one) => (one as CSSAnimation).animationName)).not.toContain("bf-sway");
});

test("however many drones work, none turns its rotors", async () => {
  await motion();
  const ship = scenarioNamed("cockpit-ship-flightdeck")!;
  const extra: Session[] = Array.from({ length: 20 }, (_, i) => ({
    id: `x${i}`,
    title: `Working ${i}`,
    manifestId: repository().manifest!.id,
    turn: { state: "working" },
    lastTurn: "14:00",
    lastTurnAt: new Date().toISOString(),
    standing: "working",
    rows: [],
    attachments: [],
  }));
  const app = mount({ ...ship, draft: { ...ship.draft, sessions: (control, more = []) => ship.draft!.sessions!(control, [...more, ...extra]) } });
  await onScreen();
  await controls().getByRole("button", { name: "Map" }).click();
  await expect.poll(() => drones().filter((one) => one.hasAttribute("data-spin")).length).toBeGreaterThan(8);
  await new Promise((done) => window.setTimeout(done, 900));
  void app;

  const turning = drones().filter((one) => one.getAnimations({ subtree: true }).some((a) => a instanceof CSSAnimation && a.animationName === "bf-spin" && a.playState === "running"));
  expect(turning).toEqual([]);
});
