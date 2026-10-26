// The Autopilot panel is off until Settings, Features turns on `features.autopilotPanel`: the Cockpit does not draw it. Turned on, it is one row, its name and its switch,
// and it appears and goes again without a reload. Through `App` on the flight deck's ship.

import { expect, test } from "vitest";
import { page } from "vitest/browser";

import { mount, onScreen, unmountAfterEach } from "./testing";

unmountAfterEach();

const list = () => page.getByRole("region", { name: "Jobs", exact: true });
const panel = () => document.querySelector<HTMLElement>(".deck-autopilot");
const KEY = "features.autopilotPanel";

async function deck() {
  const app = mount("cockpit-ship-flightdeck");
  await onScreen();
  await expect.element(list()).toBeVisible();
  // The foot is drawn: Flight control stands in it, so an Autopilot panel that is missing is not a foot that has not come.
  await expect.element(page.getByRole("region", { name: "Flight controls" })).toBeVisible();
  return app;
}

test("the setting is off by default, and the Cockpit draws no Autopilot panel", async () => {
  const app = await deck();
  const answer = await app.api.syncState!();
  const row = "full" in answer ? (answer.full as { settings: { settings: { key: string; value: unknown }[] } | null }).settings?.settings.find((one) => one.key === KEY) : undefined;
  expect(row?.value).toBe(false);
  expect(panel()).toBeNull();
  expect(page.getByRole("region", { name: "Autopilot", exact: true }).query()).toBeNull();
});

test("turned on, the panel appears with no reload, as one row: its name and its switch", async () => {
  const app = await deck();
  await app.api.saveSettings({ [KEY]: true });
  await expect.poll(panel).not.toBeNull();

  const region = page.getByRole("region", { name: "Autopilot", exact: true });
  await expect.element(region.getByRole("switch", { name: "Autopilot" })).toBeVisible();
  // Nothing else is in it: no bar, no button, no repository or state.
  expect(region.getByRole("progressbar").query()).toBeNull();
  expect(region.getByRole("button").query()).toBeNull();
  expect(region.getByRole("switch").elements()).toHaveLength(1);
  // One row: the panel is no taller than its band.
  const box = panel()!.getBoundingClientRect();
  const band = panel()!.querySelector(".deck-band")!.getBoundingClientRect();
  expect(box.height).toBeLessThanOrEqual(band.height + 8);
});

test("turned off again, the panel goes with no reload", async () => {
  const app = await deck();
  await app.api.saveSettings({ [KEY]: true });
  await expect.poll(panel).not.toBeNull();
  await app.api.saveSettings({ [KEY]: false });
  await expect.poll(panel).toBeNull();
});
