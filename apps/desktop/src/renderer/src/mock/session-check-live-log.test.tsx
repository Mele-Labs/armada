// A Session's running Check, opened from its thread: the log grows while it runs, and each of the
// panel's two buttons asks for its own stop. The walk `session-check-live-log` draws it.

import { expect, test } from "vitest";
import { page, userEvent } from "vitest/browser";

import { stopsAsked } from "./session-check-stops";
import { mount, onScreen, toSession, unmountAfterEach } from "./testing";

unmountAfterEach();

async function openRunning() {
  mount("session-check-live-log");
  await onScreen();
  await toSession("Session check rows");
  await userEvent.click(page.getByRole("region", { name: "Thread" }).getByRole("button", { name: "Check app_smoke, running" }));
  const log = page.getByRole("dialog", { name: "Check log" });
  await expect.element(log).toBeVisible();
  return log;
}

test("a running Session check's panel shows lines that grow while it runs", async () => {
  const log = await openRunning();
  await expect.element(log.getByText(/RUN {2}v3\.2\.4/)).toBeVisible();
  await expect.element(log.getByRole("img", { name: "Being written" })).toBeVisible();
  // Not there when it opened; printed while it is open.
  expect(log.getByText(/walks\.test\.tsx \(12 tests\)/).query()).toBeNull();
  await expect.element(log.getByText(/walks\.test\.tsx \(12 tests\)/), { timeout: 8_000 }).toBeVisible();
  await expect.element(log.getByText(/sessions\.test\.tsx \(4 tests\)/), { timeout: 8_000 }).toBeVisible();
});

test("Stop and Force stop each ask for their own stop of the run", async () => {
  const log = await openRunning();
  await userEvent.click(log.getByRole("button", { name: "Stop", exact: true }));
  expect(stopsAsked).toEqual([{ run: 31, force: false }]);
  await userEvent.click(log.getByRole("button", { name: "Force stop", exact: true }));
  expect(stopsAsked).toEqual([
    { run: 31, force: false },
    { run: 31, force: true },
  ]);
});
