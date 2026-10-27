// Settings, Bridge, Spacing (`spacing.density`): Compact and Roomy put their token set on the root with no reload, and Default takes it
// off again. Flight control's keys keep Default's height in Compact, where the smaller steps would cut their names. Through `App` on the
// flight deck's ship.

import { expect, test } from "vitest";
import { page } from "vitest/browser";

import { mount, onScreen, unmountAfterEach } from "./testing";

unmountAfterEach();

const KEY = "spacing.density";
const root = () => document.documentElement;
const step = () => getComputedStyle(root()).getPropertyValue("--space-1").trim();
const key = () => document.querySelector<HTMLElement>(".deck-controls__block .deck-tile")!.getBoundingClientRect().height;

async function deck() {
  const app = mount("cockpit-ship-flightdeck");
  await onScreen();
  await expect.element(page.getByRole("region", { name: "Flight controls" })).toBeVisible();
  return app;
}

test("Default ships, and the root carries no spacing", async () => {
  const app = await deck();
  const answer = await app.api.syncState!();
  const row = "full" in answer ? (answer.full as { settings: { settings: { key: string; value: unknown }[] } | null }).settings?.settings.find((one) => one.key === KEY) : undefined;
  expect(row?.value).toBe("default");
  expect(root().dataset["spacing"]).toBeUndefined();
  expect(step()).toBe("4px");
});

test("Compact and Roomy change the steps with no reload, and Default puts them back", async () => {
  const app = await deck();
  await app.api.saveSettings({ [KEY]: "compact" });
  await expect.poll(() => root().dataset["spacing"]).toBe("compact");
  expect(step()).toBe("3px");

  await app.api.saveSettings({ [KEY]: "roomy" });
  await expect.poll(() => root().dataset["spacing"]).toBe("roomy");
  expect(step()).toBe("5px");

  await app.api.saveSettings({ [KEY]: "default" });
  await expect.poll(() => root().dataset["spacing"]).toBeUndefined();
  expect(step()).toBe("4px");
});

test("Flight control's keys are as tall in Compact as in Default", async () => {
  const app = await deck();
  const shipped = key();
  await app.api.saveSettings({ [KEY]: "compact" });
  await expect.poll(() => root().dataset["spacing"]).toBe("compact");
  expect(key()).toBe(shipped);
  await app.api.saveSettings({ [KEY]: "default" });
});
