// With a Job open on the flight deck's stage, the flight controls are that Session's: Report hands off to a new Session, Auto land is Fleet's flag on the Session
// and sends the agent its words, and Release is one of them. Activity gives its place to the Job's own panels and returns when it closes. The brief is
// the owner's ask of 15 Oct 2026.

import { beforeEach, expect, test } from "vitest";
import { page } from "vitest/browser";

import { mount, onScreen, unmountAfterEach } from "./testing";

unmountAfterEach();
// Whether the row is expanded is kept in the window's storage, so one test would hand it to the next.
beforeEach(() => localStorage.clear());

const list = () => page.getByRole("region", { name: "Jobs", exact: true });
const controls = () => page.getByRole("region", { name: "Flight controls" });
const pane = () => page.getByRole("region", { name: "Job: Pin the store clock" });
// Activity names each event on its dot.
const feed = () => [...document.querySelectorAll(".deck-activity .armada-tl__dot")].map((one) => one.getAttribute("aria-label")).join("\n");
const tiles = () => [...document.querySelectorAll(".deck-controls .deck-tile")].map((one) => one.getAttribute("aria-label"));

async function open() {
  mount("cockpit-ship-flightdeck");
  await onScreen();
  await expect.element(list()).toBeVisible();
  await list().getByRole("button", { name: "Pin the store clock", exact: true }).click();
  await expect.element(pane()).toBeVisible();
}

test("with a Job open the flight controls are that Session's, and the four return when it closes", async () => {
  await open();
  await expect.poll(tiles).toEqual(["Auto land", "Pause", "Judge", "Report issue with job", "Release everything the Job holds", "Terminal", "Browser", "Requests"]);
  await pane().getByRole("button", { name: "Back" }).click();
  await expect.poll(tiles).toEqual(["Checks", "Needs me", "Map", "Timeline"]);
});

test("Report is offered where the Session's repository is known", async () => {
  await open();
  await expect.element(controls().getByRole("button", { name: "Report issue with job" })).not.toHaveAttribute("aria-disabled");
});

test("Auto land reads Off, then On once Fleet keeps it, and sends the Session the words for each", async () => {
  await open();
  const auto = () => controls().getByRole("button", { name: "Auto land" });
  await expect.element(auto()).toHaveAttribute("aria-pressed", "false");
  await expect.element(auto().getByRole("img", { name: "Auto land off" })).toBeVisible();

  await auto().click();
  await expect.poll(() => pane().element().textContent?.includes("Fleet turns on auto-merge when your pull request opens")).toBe(true);
  await expect.element(auto()).toHaveAttribute("aria-pressed", "true");
  await expect.element(auto().getByRole("img", { name: "Auto land on" })).toBeVisible();

  await auto().click();
  await expect.poll(() => pane().element().textContent?.includes("Auto land is off. Fleet turns auto-merge off.")).toBe(true);
  await expect.element(auto()).toHaveAttribute("aria-pressed", "false");
  await expect.element(auto().getByRole("img", { name: "Auto land off" })).toBeVisible();
});

test("with a Job open Activity gives its place to the Job's own panels, and returns with all its events when it closes", async () => {
  mount("cockpit-ship-flightdeck");
  await onScreen();
  await expect.element(list()).toBeVisible();
  const other = "Migration notes";
  await expect.poll(() => feed().includes(other)).toBe(true);

  await list().getByRole("button", { name: "Pin the store clock", exact: true }).click();
  await expect.element(pane()).toBeVisible();
  await expect.element(page.getByRole("region", { name: "Activity", exact: true })).not.toBeInTheDocument();

  await pane().getByRole("button", { name: "Back" }).click();
  await expect.poll(() => feed().includes(other)).toBe(true);
});

// The row is drawn at the deck's own size, so a Job with seven tiles holds more than it shows; Expand is there to wrap them, and Collapse to put them back.
const row = () => document.querySelector<HTMLElement>(".deck-flight")!;

test("Expand appears with a Job open whose tiles overflow, wraps them, and Collapse puts them back in one row", async () => {
  await open();
  await expect.poll(() => row().scrollWidth > row().clientWidth).toBe(true);
  expect(row().hasAttribute("data-expanded")).toBe(false);
  const tops = () => new Set([...row().querySelectorAll(".deck-tile")].map((one) => Math.round(one.getBoundingClientRect().top)));
  expect(tops().size).toBe(1);

  await controls().getByRole("button", { name: "Expand", exact: true }).click();
  await expect.element(controls().getByRole("button", { name: "Collapse", exact: true })).toHaveAttribute("aria-expanded", "true");
  expect(row().hasAttribute("data-expanded")).toBe(true);
  await expect.poll(() => tops().size).toBeGreaterThan(1);

  await controls().getByRole("button", { name: "Collapse", exact: true }).click();
  await expect.element(controls().getByRole("button", { name: "Expand", exact: true })).toHaveAttribute("aria-expanded", "false");
  expect(row().hasAttribute("data-expanded")).toBe(false);
  await expect.poll(() => tops().size).toBe(1);
});

test("closing the Job takes Collapse away, with the Job's tiles", async () => {
  await open();
  await controls().getByRole("button", { name: "Expand", exact: true }).click();
  await pane().getByRole("button", { name: "Back" }).click();
  await expect.poll(tiles).toEqual(["Checks", "Needs me", "Map", "Timeline"]);
  expect(controls().element().querySelector('[aria-label="Expand"], [aria-label="Collapse"]')).toBeNull();
});
