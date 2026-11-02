// The console through `App`: Calls and Activity open a HUD over the field and close back onto their facet, the rack seats an instrument by
// keyboard, and where the canvas does not start the buttons stand as plain keys. The look is the walk's (`walks/bays-look-like-bays.ts`).

import { beforeEach, expect, test, vi } from "vitest";
import { page, userEvent } from "vitest/browser";

import { ConsoleStage } from "../cockpit/deck/pixi/console-stage";
import { mount, onScreen, unmountAfterEach } from "./testing";

unmountAfterEach();
// The seated bays are kept in the window's storage, so one test would hand them to the next.
beforeEach(() => localStorage.clear());

const controls = () => page.getByRole("region", { name: "Flight controls" });
const facet = (name: string) => controls().getByRole("button", { name, exact: true });
const hud = (kind: string) => document.querySelector(`[data-hud="${kind}"]`);
const calls = () => page.getByRole("region", { name: "Calls", exact: true });
const summary = () => document.querySelector(".console-calls__so-far")?.textContent ?? null;
const named = () => document.querySelector('[data-hud="calls"] .console-hud__name')?.textContent ?? "";
const seated = () => JSON.parse(localStorage.getItem("armada.console.bays") ?? "null") as string[] | null;

async function deck(scenario = "cockpit-ship-flightdeck") {
  const app = mount(scenario);
  await onScreen();
  await expect.element(controls()).toBeVisible();
  await expect.element(facet("Calls")).toBeInTheDocument();
  return app;
}

test("pressing Calls opens the HUD on the question, reads pressed, and a second press closes it", async () => {
  await deck();
  await expect.element(facet("Calls")).toHaveAttribute("aria-pressed", "false");
  await facet("Calls").click();
  await expect.element(calls().getByRole("radio", { name: /^Hash/ })).toBeVisible();
  await expect.element(facet("Calls")).toHaveAttribute("aria-pressed", "true");
  await facet("Calls").click();
  await expect.poll(() => hud("calls")).toBeNull();
});

test("answering a call moves the HUD to the next one", async () => {
  await deck();
  await facet("Calls").click();
  await expect.element(calls().getByRole("radio", { name: /^Hash/ })).toBeVisible();
  const first = named();
  await expect.poll(() => document.activeElement?.closest(".armada-needs-panel")).not.toBeNull();
  await userEvent.keyboard("1");
  await calls().getByRole("button", { name: /^Answer/ }).click();
  await expect.poll(named).not.toBe(first);
  expect(hud("calls")).not.toBeNull();
});

test("Next call shows the next Job calling and comes round to the first", async () => {
  await deck();
  await facet("Calls").click();
  await expect.element(calls().getByRole("button", { name: /^Next call/ })).toBeVisible();
  const first = named();
  const seen = new Set([first]);
  for (let i = 0; i < 10; i += 1) {
    const was = named();
    await calls().getByRole("button", { name: /^Next call/ }).click();
    await expect.poll(named).not.toBe(was);
    if (named() === first) break;
    seen.add(named());
  }
  expect(named()).toBe(first);
  expect(seen.size).toBeGreaterThan(1);
});

test("Esc closes the Calls HUD and gives the keys back to its facet", async () => {
  await deck();
  await facet("Calls").click();
  await expect.element(calls().getByRole("radio", { name: /^Hash/ })).toBeVisible();
  await userEvent.keyboard("{Escape}");
  await expect.poll(() => hud("calls")).toBeNull();
  expect(document.activeElement).toBe(facet("Calls").element());
});

test("⌥⇧2, the Calls key, opens the HUD and closes it, and does not jump to a row", async () => {
  await deck();
  const selected = () => document.querySelector(".deck-row[data-selected]");
  await userEvent.keyboard("{Alt>}{Shift>}2{/Shift}{/Alt}");
  await expect.element(calls().getByRole("radio", { name: /^Hash/ })).toBeVisible();
  await expect.element(facet("Calls")).toHaveAttribute("aria-pressed", "true");
  await userEvent.keyboard("{Alt>}{Shift>}2{/Shift}{/Alt}");
  await expect.poll(() => hud("calls")).toBeNull();
  expect(document.activeElement).toBe(facet("Calls").element());
  // The row was lit only for the HUD: nothing was opened.
  expect(selected()).toBeNull();
});

test("the Calls facet is named Calls", async () => {
  await deck();
  await expect.element(facet("Calls")).toBeInTheDocument();
  expect(facet("Needs me").query()).toBeNull();
});

test("pressing Activity opens its HUD, reads pressed, and Esc returns to the facet", async () => {
  await deck();
  await facet("Activity").click();
  await expect.element(page.getByRole("dialog", { name: "Activity" })).toBeVisible();
  await expect.poll(() => hud("activity")?.querySelectorAll(".armada-tl__dot").length ?? 0).toBeGreaterThan(0);
  await expect.element(facet("Activity")).toHaveAttribute("aria-pressed", "true");
  await userEvent.keyboard("{Escape}");
  await expect.poll(() => hud("activity")).toBeNull();
  expect(document.activeElement).toBe(facet("Activity").element());
  await expect.element(facet("Activity")).toHaveAttribute("aria-pressed", "false");
});

test("an instrument is seated in a bay from the rack by keyboard, and Esc takes an edit back", async () => {
  await deck();
  await page.elementLocator(document.querySelector(".deck-controls .deck-band [aria-label='Edit']")!).click();
  const socket = () => page.getByRole("button", { name: /^Bay 1/ });
  await expect.element(socket()).toHaveAttribute("aria-label", "Bay 1, Checks");
  expect(document.activeElement).toBe(socket().element());
  // No sample instrument is on offer, and nothing makes one.
  expect(page.getByRole("option", { name: /Plan usage/ }).query()).toBeNull();
  expect(controls().getByRole("button", { name: /Make one/ }).query()).toBeNull();

  await userEvent.keyboard("{Enter}");
  await expect.poll(() => document.activeElement?.closest(".console-tray")).not.toBeNull();
  const instrument = () => document.activeElement!.getAttribute("aria-label");
  while (instrument() !== null && !/Needles|Gauges/.test(instrument()!)) await userEvent.keyboard("{ArrowRight}");
  const chosen = instrument()!;
  await userEvent.keyboard("{Enter}");
  await expect.element(socket()).toHaveAttribute("aria-label", `Bay 1, ${chosen}`);
  expect(seated()?.[0]).toBe("needles");

  await userEvent.keyboard("{Escape}");
  await expect.poll(() => seated()?.[0]).toBe("matrix");
});

test("a Job with a summary draws it", async () => {
  await deck();
  await facet("Calls").click();
  await expect.poll(summary).toContain("Read the manifest loader");
});

test("a Job with no summary draws no summary", async () => {
  await deck("session-waiting-on-you");
  await facet("Calls").click();
  await expect.element(calls().getByRole("button", { name: "Open Job" })).toBeVisible();
  expect(document.querySelector(".console-calls__side")).not.toBeNull();
  expect(summary()).toBeNull();
});

test("folded, the console's band names it and a press on it opens it again", async () => {
  localStorage.setItem("armada.panel-collapsed.controls", "1");
  mount("cockpit-ship-flightdeck");
  await onScreen();
  // The band keeps no name once the console is open, so it is found by its place.
  await expect.poll(() => document.querySelector(".deck-controls > .deck-band")).not.toBeNull();
  const band = document.querySelector<HTMLElement>(".deck-controls > .deck-band")!;
  expect(band.getAttribute("aria-expanded")).toBe("false");
  expect(band.querySelector(".deck-band__name")!.getBoundingClientRect().width).toBeGreaterThan(0);
  band.click();
  await expect.poll(() => band.getAttribute("aria-expanded")).toBe("true");
});

test("where the canvas does not start, the buttons stand as plain keys and still press", async () => {
  vi.spyOn(console, "error").mockImplementation(() => undefined);
  const start = vi.spyOn(ConsoleStage.prototype as unknown as { init(): Promise<void> }, "init").mockRejectedValue(new Error("no hardware WebGL"));
  try {
    await deck();
    await expect.poll(() => document.querySelector(".console-deck")?.hasAttribute("data-no-canvas")).toBe(true);
    const box = facet("Calls").element().getBoundingClientRect();
    expect(box.width).toBeGreaterThan(40);
    expect(box.height).toBeGreaterThan(24);
    expect(getComputedStyle(facet("Calls").element()).opacity).not.toBe("0");
    await facet("Calls").click();
    await expect.element(calls().getByRole("radio", { name: /^Hash/ })).toBeVisible();
  } finally {
    start.mockRestore();
    vi.restoreAllMocks();
  }
});
