// The mock's own scenario picker, through `mountPicker` — the mount the page
// uses, so what is asserted here is the control the owner presses.
//
// **The picker is not part of the app**, but it is drawn inside it: its root
// is a second one beside the app's, and what it renders goes into the app's
// own left column through a portal. So a test here puts up both.
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
import { mount, onScreen, unmountAfterEach } from "./testing";

unmountAfterEach();

const SCENARIO = "arc/executing-concurrent";

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
  mount("every-state");
  await onScreen();
  host = document.createElement("div");
  host.id = "picker";
  document.body.append(host);
  takePicker = mountPicker(SCENARIO, host);
  await expect.element(said()).toBeVisible();
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

const said = () => page.getByRole("button", { name: `Mock scenario — ${SCENARIO}` });
const field = () => page.getByRole("textbox", { name: "Find a scenario" });
const rows = () => [...document.querySelectorAll<HTMLAnchorElement>(".armada-mock-picker__row")];
const names = () => rows().map((row) => row.textContent ?? "");

/** What the picker drew, so a claim about where it is can be made about the element itself. */
const pickerEl = (): HTMLElement => document.querySelector<HTMLElement>(".armada-mock-picker")!;

/** Typed into the field, replacing whatever was there. */
async function find(query: string): Promise<void> {
  await userEvent.fill(field().element() as HTMLInputElement, query);
}

test("the picker is in the app's left column, not over the content", async () => {
  await show();

  // Not a coordinate: being a child of the column is what makes it impossible
  // for it to cover anything, and a box that happens to miss the content today
  // would still be a layer over it.
  expect(pickerEl().closest(".armada-shell__left")).not.toBeNull();
  expect(document.getElementById("picker")?.children.length).toBe(0);
  expect(getComputedStyle(pickerEl()).position).not.toBe("fixed");
});

test("it says which scenario is on without being opened", async () => {
  await show();
  await expect.element(said()).toHaveTextContent(SCENARIO);
});

test("typing part of a name narrows to it", async () => {
  await show();
  await said().click();
  await expect.element(field()).toBeVisible();

  // Every scenario is listed before a word is typed — the list is the roster.
  expect(rows().length).toBe(SCENARIOS.length);

  await find("fleetnot");
  expect(names().length).toBeLessThan(SCENARIOS.length);
  expect(names()[0]).toBe("fleet-not-running");
});

test("every row is a line tall, however many there are", async () => {
  await show();
  await said().click();
  await expect.element(field()).toBeVisible();

  // 1 Oct 2026: the open list drew as one thin bar. Each row is a flex item
  // with `overflow: hidden`, so the list shrank all of them to their padding.
  const line = parseFloat(getComputedStyle(rows()[0]!).lineHeight);
  for (const row of rows()) expect(row.getBoundingClientRect().height).toBeGreaterThanOrEqual(line);
});

test("the search is fuzzy — the initials of a long name reach it", async () => {
  await show();
  await said().click();

  // Nothing in `arc/executing-concurrent` spells `arcex`; every letter of it
  // is in that name in that order, which is the whole ask.
  await find("arcex");
  expect(names()).toContain(SCENARIO);
  for (const one of names()) expect(one).toMatch(/a.*r.*c.*e.*x/);
});

test("choosing one mounts it — the reload `?scenario=` is read on", async () => {
  await show();
  const went = intercept();

  await said().click();
  await find("fleetnot");
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
  await expect.element(page.getByText("Fleet is not running").first()).toBeVisible();
  app.unmount();
  host2.remove();
});

test("Enter takes the top row, so a narrowed query is two keys", async () => {
  await show();
  const went = intercept();

  await said().click();
  await find("fleetnot");
  await userEvent.keyboard("{Enter}");

  expect(new URL(went[0]!).searchParams.get("scenario")).toBe("fleet-not-running");
});

test("Esc gives it up, and the scenario is still readable", async () => {
  await show();
  await said().click();
  await expect.element(field()).toBeVisible();

  await userEvent.keyboard("{Escape}");
  expect(field().query()).toBeNull();
  await expect.element(said()).toHaveTextContent(SCENARIO);
});
