// The mock's own scenario finder, through `mountPicker` — the mount the page uses. It has no button: the
// palette's "Mock scenario" row (⌘K) opens it as a centred layer.
//
// **The finder is not part of the app**, but it fills a row of the app's palette (`palette-extra.ts`), so a
// test here puts up both.
//
// The owner, 30 Sep 2026: *"I still would like to find a better way to overlay
// the mock scenarios. What if we put it in the left side panel? Also I would
// love other be able to fuzzy search the scenario so I can quickly select
// one."*

import { afterEach, expect, test } from "vitest";
import { page, userEvent } from "vitest/browser";

import "../styles/index.css";
import { mountPicker } from "./Picker";
import { mountApp } from "./mount";
import { SCENARIOS } from "./scenario";
import { EVERY_WALK } from "./walks";
import { mount, onScreen, unmountAfterEach } from "./testing";

unmountAfterEach();

const SCENARIO = "merge-line-failed-check";

let takePicker: (() => void) | null = null;
let host: HTMLElement | null = null;
let stopIntercepting: (() => void) | null = null;

afterEach(() => {
  takePicker?.();
  host?.remove();
  stopIntercepting?.();
  takePicker = null;
  host = null;
  stopIntercepting = null;
});

/** The app up, and the picker mounted beside it as `main.tsx` mounts the pair. */
async function show(): Promise<void> {
  mount("default");
  await onScreen();
  host = document.createElement("div");
  host.id = "picker";
  document.body.append(host);
  takePicker = mountPicker(SCENARIO, host);
}

/** ⌘K, then the palette's Mock scenario row. */
async function openFinder(): Promise<void> {
  await userEvent.click(page.getByRole("button", { name: /Search, or describe work/ }));
  await userEvent.fill(page.getByRole("dialog", { name: "Command palette" }).getByRole("combobox").element() as HTMLInputElement, "Mock scenario");
  await userEvent.keyboard("{Enter}");
  await expect.element(field()).toBeVisible();
}

/**
 * Where a press would have taken the page. **A row is a link**, so this is the
 * navigation itself held back rather than a stand-in for it — `window.location`
 * cannot be replaced in a browser, and a real reload would take the test runner
 * with it.
 */
function intercept(): string[] {
  const went: string[] = [];
  function held(event: MouseEvent): void {
    const link = (event.target as Element | null)?.closest?.("a");
    if (link === null || link === undefined) return;
    event.preventDefault();
    went.push(link.href);
  }
  document.addEventListener("click", held, true);
  stopIntercepting = () => document.removeEventListener("click", held, true);
  return went;
}

const field = () => page.getByRole("textbox", { name: "Find a scenario" });
const rows = () => [...document.querySelectorAll<HTMLAnchorElement>(".armada-mock-picker__row")];
const names = () => rows().map((row) => row.textContent ?? "");

/** Typed into the field, replacing whatever was there. */
async function find(query: string): Promise<void> {
  await userEvent.fill(field().element() as HTMLInputElement, query);
}

test("the title row has no scenario button, and the palette lists the finder", async () => {
  await show();
  expect(document.querySelector('.armada-title-bar [aria-label^="Mock scenario"]')).toBeNull();
  expect(document.getElementById("picker")?.children.length).toBe(0);

  await userEvent.click(page.getByRole("button", { name: /Search, or describe work/ }));
  await userEvent.fill(page.getByRole("dialog", { name: "Command palette" }).getByRole("combobox").element() as HTMLInputElement, "Mock scenario");
  await expect.element(page.getByRole("option", { name: /^Mock scenario/ })).toBeVisible();
});

test("the finder is a centred layer, closed until the row runs", async () => {
  await show();
  expect(field().query()).toBeNull();
  await openFinder();
  const layer = document.querySelector<HTMLElement>(".armada-mock-picker__layer")!;
  expect(getComputedStyle(layer).position).toBe("fixed");
  const box = layer.getBoundingClientRect();
  expect(Math.abs(box.left + box.width / 2 - window.innerWidth / 2)).toBeLessThan(2);
});

test("typing part of a name narrows to it", async () => {
  await show();
  await openFinder();

  // Every scenario and every walk is listed before a word is typed — the list is the roster.
  expect(rows().length).toBe(SCENARIOS.length + EVERY_WALK.size);

  await find("fleet-not-running");
  expect(names().length).toBeLessThan(SCENARIOS.length + EVERY_WALK.size);
  expect(names()[0]).toBe("fleet-not-running");
});

test("every row is a line tall, however many there are", async () => {
  await show();
  await openFinder();

  // 1 Oct 2026: the open list drew as one thin bar. Each row is a flex item
  // with `overflow: hidden`, so the list shrank all of them to their padding.
  const line = parseFloat(getComputedStyle(rows()[0]!).lineHeight);
  for (const row of rows()) expect(row.getBoundingClientRect().height).toBeGreaterThanOrEqual(line);
});

test("the search is fuzzy — the initials of a long name reach it", async () => {
  await show();
  await openFinder();

  // Nothing in `merge-line-failed-check` spells `mlfch`; every letter of it
  // is in that name in that order, which is the whole ask.
  await find("mlfch");
  expect(names()).toContain(SCENARIO);
  expect(names().length).toBeLessThan(SCENARIOS.length + EVERY_WALK.size);
});

test("choosing one mounts it — the reload `?scenario=` is read on", async () => {
  await show();
  const went = intercept();

  await openFinder();
  await find("fleet-not-running");
  await page.getByRole("link", { name: "fleet-not-running" }).click();

  expect(went).toHaveLength(1);
  const asked = new URL(went[0]!).searchParams.get("scenario");
  expect(asked).toBe("fleet-not-running");

  // And the page that reload lands on is the app on that scenario — the other
  // half of the claim, which a URL on its own does not make.
  const host2 = document.createElement("div");
  document.body.append(host2);
  const app = mountApp(asked!, host2);
  await app.onScreen;
  // The Cockpit powers on over a Fleet that is down, so the title row's Fleet button is what names it (`cockpit/boot.ts`).
  await expect.element(page.getByRole("button", { name: "Fleet — Not running" })).toBeVisible();
  app.unmount();
  host2.remove();
});

test("a walk is a row too, and choosing one plays it — the reload `?walk=` is read on", async () => {
  await show();
  const went = intercept();

  await openFinder();
  await find("cockpit sessions list");
  await page.getByRole("link", { name: "cockpit sessions list" }).click();

  expect(went).toHaveLength(1);
  const asked = new URL(went[0]!).searchParams;
  expect(asked.get("walk")).toBe("cockpitSessionsList");
  // `?walk` wins over `?scenario` already; the stale one is not left to say otherwise.
  expect(asked.has("scenario")).toBe(false);
});

test("Enter takes the top row, so a narrowed query is two keys", async () => {
  await show();
  const went = intercept();

  await openFinder();
  await find("fleet-not-running");
  await userEvent.keyboard("{Enter}");

  expect(new URL(went[0]!).searchParams.get("scenario")).toBe("fleet-not-running");
});

test("Esc gives it up", async () => {
  await show();
  await openFinder();

  await userEvent.keyboard("{Escape}");
  expect(field().query()).toBeNull();
});

test("walks are listed first, in words, above the scenarios", async () => {
  await show();
  await openFinder();

  const all = rows();
  expect(all.slice(0, EVERY_WALK.size).every((row) => row.dataset.walk !== undefined)).toBe(true);
  expect(all[EVERY_WALK.size]?.dataset.walk).toBeUndefined();
  expect(names()).toContain("cockpit sessions list");
  expect(names()).not.toContain("cockpitSessionsList");
});

test("on a walk page the finder marks the walk's row", async () => {
  mount("default");
  await onScreen();
  host = document.createElement("div");
  document.body.append(host);
  takePicker = mountPicker("cockpitSessionsList", host, true);
  await openFinder();
  const current = document.querySelectorAll<HTMLAnchorElement>(".armada-mock-picker__row[data-current]");
  expect(current).toHaveLength(1);
  expect(current[0]?.textContent).toBe("cockpit sessions list");
  expect(current[0]?.href).toContain("walk=cockpitSessionsList");
});

test("every scenario row has a name: a plain value a scenarios file exports is not one", () => {
  expect(SCENARIOS.every((one) => typeof one.name === "string")).toBe(true);
});
