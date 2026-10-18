// A restart Fleet reports as failed raises the failure toast with Dispatch job, once. The same failed report read again
// raises nothing, a dismissed toast stays dismissed, and a restart that fails the same way after another press is a new one.

import { beforeEach, expect, test } from "vitest";
import { page } from "vitest/browser";

import { s274BuildFailedToast } from "./scenarios/build-failed-toast";
import type { Scenario } from "./scenario";
import { timePasses } from "./time-passes";
import { mount, onScreen, unmountAfterEach } from "./testing";

unmountAfterEach();
beforeEach(() => window.localStorage.clear());

const LINE = "error[E0432]: unresolved import crate::drones";
const failed = () => ({ fleetBuild: { on: "main" as const, commit: "8944d2f", failed: LINE } });
const restarting = () => ({ fleetBuild: { on: "main" as const, commit: "8944d2f", restarting: "preview" as const } });

const dispatch = () => page.getByRole("button", { name: "Dispatch job" });
const toast = () => page.getByText(`Preview build failed: ${LINE}`);
const settled = () => new Promise((done) => window.setTimeout(done, 250));

async function window_(later: Scenario["later"]) {
  mount({ ...s274BuildFailedToast, later });
  await onScreen();
}

test("a restart that fails raises the toast with Dispatch job", async () => {
  await window_([failed()]);
  expect(dispatch().query()).toBeNull();
  await settled();
  timePasses();
  await expect.element(toast()).toBeVisible();
  await expect.element(dispatch()).toBeVisible();
});

test("the same failed report read again raises no second toast", async () => {
  await window_([failed(), failed(), failed()]);
  timePasses();
  await expect.element(dispatch()).toBeVisible();
  timePasses();
  timePasses();
  await settled();
  expect(dispatch().elements()).toHaveLength(1);
});

test("a dismissed toast stays dismissed on a re-read", async () => {
  await window_([failed(), failed()]);
  timePasses();
  await page.getByRole("button", { name: "Dismiss" }).click();
  expect(dispatch().query()).toBeNull();
  timePasses();
  await settled();
  expect(dispatch().query()).toBeNull();
});

test("a failure after another press is a new one", async () => {
  await window_([failed(), restarting(), failed()]);
  timePasses();
  await page.getByRole("button", { name: "Dismiss" }).click();
  timePasses();
  timePasses();
  await expect.element(dispatch()).toBeVisible();
});

test("a failure Bridge was reopened on raises once, and not again after it was seen", async () => {
  mount({ ...s274BuildFailedToast, state: { ...s274BuildFailedToast.state, ...failed() }, later: [] });
  await onScreen();
  await expect.element(dispatch()).toBeVisible();
  expect(window.localStorage.getItem("armada.bridge.build-failure-raised")).toBe(`8944d2f ${LINE}`);
});
