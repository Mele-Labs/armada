// The flight deck in the real Cockpit, through `App` on the ship scenario: a call opens as a pane and leaves Incoming, a
// Session that replied raises a call, the Cockpit tab comes back to the deck as it was left, Checks narrows what the deck
// shows, and with the switch off the classic Cockpit and its rail are what is drawn. #2301.

import { expect, test } from "vitest";
import { page, userEvent } from "vitest/browser";

import { setDeckPreview } from "../cockpit/deck/preview";
import { mount, onScreen, unmountAfterEach } from "./testing";
import { timePasses } from "./time-passes";

unmountAfterEach();

const list = () => page.getByRole("region", { name: "Jobs and Sessions" });
const incoming = () => page.getByRole("region", { name: "Incoming calls" });
const controls = () => page.getByRole("region", { name: "Flight controls" });
const places = () => page.getByRole("navigation", { name: "Places" });
const stage = () => document.querySelector(".deck-stage > .deck-pane");
const hails = () => [...document.querySelectorAll<HTMLElement>(".deck-incoming .deck-hail")].map((one) => one.getAttribute("aria-label") ?? "");

async function deck() {
  const app = mount("cockpit-ship-flightdeck");
  await onScreen();
  await expect.element(list()).toBeVisible();
  return app;
}

test("calls open as panes and leave Incoming", async () => {
  await deck();
  await expect.poll(() => hails().length).toBeGreaterThan(0);
  const first = hails()[0]!;
  const before = hails().length;
  await incoming().getByRole("button", { name: first }).click();

  // What it is about opens on the stage, and opening it was the visit.
  await expect.poll(() => hails().includes(first)).toBe(false);
  expect(hails()).toHaveLength(before - 1);
  expect(stage()?.getAttribute("aria-label")).not.toBe("Fleet timeline");
  expect(stage()?.getAttribute("aria-label")).toContain(first.replace(/^[^:]+: /, ""));
});

test("a replied session raises an Incoming call", async () => {
  await deck();
  const title = "Pin the store clock";
  await expect.poll(() => hails().length).toBeGreaterThan(0);
  expect(hails().some((one) => one.includes(title))).toBe(false);

  timePasses();
  await expect.element(incoming().getByRole("button", { name: new RegExp(title) })).toBeVisible();
});

test("the Cockpit tab returns from a page opened from the deck exactly", async () => {
  await deck();
  await list().getByRole("button", { name: "Shorten the reconnect wait" }).click();
  const pane = page.getByRole("region", { name: /^Job: Shorten the reconnect wait/ });
  await expect.element(pane).toBeVisible();

  await pane.getByRole("button", { name: "Open the Job page" }).click();
  await expect.element(pane).not.toBeInTheDocument();
  expect(document.querySelector(".deck-list")).toBeNull();

  await places().getByRole("tab", { name: "Cockpit" }).click();
  await expect.element(page.getByRole("region", { name: /^Job: Shorten the reconnect wait/ })).toBeVisible();
  expect(document.querySelector('.deck-row[data-selected]')?.getAttribute("aria-label")).toBe("Shorten the reconnect wait");

  // Exactly, to history too: the tab went back to the deck and did not open it a second time, so Back does not find the page again.
  await userEvent.keyboard("{Meta>}[[{/Meta}");
  await new Promise((done) => window.setTimeout(done, 200));
  expect(document.querySelector(".deck-list")).not.toBeNull();
});

test("Checks filters list, Activity and Stage to subjects running a Check", async () => {
  await deck();
  const kept = "Add the pause marker column";
  const dropped = "Shorten the reconnect wait";
  const rows = () => [...document.querySelectorAll(".deck-list .deck-row")].map((one) => one.getAttribute("aria-label"));
  await expect.poll(() => rows().includes(dropped)).toBe(true);
  const whole = rows().length;

  // The feed and the stage's timeline keep the same subjects and no others: both name it before the filter, neither after.
  const feed = () => document.querySelector(".deck-activity")?.textContent ?? "";
  const timeline = () => document.querySelector('.deck-pane[aria-label="Fleet timeline"]')?.textContent ?? "";
  await expect.poll(() => feed().includes(dropped) && timeline().includes(dropped)).toBe(true);

  await controls().getByRole("button", { name: "Checks" }).click();
  await expect.poll(() => rows().includes(dropped)).toBe(false);
  expect(rows()).toContain(kept);
  expect(rows().length).toBeLessThan(whole);
  expect(feed()).not.toContain(dropped);
  expect(timeline()).not.toContain(dropped);
  await controls().getByRole("button", { name: "Checks" }).click();
  await expect.poll(() => rows().length).toBe(whole);
});

test("the switch off leaves the classic Cockpit and rail untouched", async () => {
  await deck();
  // The deck draws no rail; the places stand in the title bar.
  expect(document.querySelector('nav[aria-label="Work"]')).toBeNull();
  await expect.element(places()).toBeVisible();

  setDeckPreview(false);
  await expect.element(page.getByRole("navigation", { name: "Work" })).toBeVisible();
  expect(document.querySelector('nav[aria-label="Places"]')).toBeNull();
  expect(document.querySelector(".deck-list, .deck-bays, .deck-incoming, .deck-controls")).toBeNull();
});
