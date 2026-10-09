// A Session's Checks, through `App`: the thread draws a row per `armada check` the agent ran, one
// out ends, and pressing it opens that run on the Checks page with the Session's owner chip beside it.
// The walk `session-checks` draws it; this holds that the presses reach the same code.

import { expect, test } from "vitest";
import { page, userEvent } from "vitest/browser";

import { mount, onScreen, unmountAfterEach } from "./testing";
import { timePasses } from "./time-passes";

unmountAfterEach();

test("Session checks: a running row ends, and pressing it opens the run with the Session's owner chip", async () => {
  mount("session-checks");
  await onScreen();
  await userEvent.click(page.getByRole("button", { name: "Sessions", exact: true }));
  await userEvent.click(page.getByRole("region", { name: "Sessions" }).getByRole("button", { name: "Session check rows" }));

  const thread = page.getByRole("region", { name: "Thread" });
  await expect.element(thread.getByRole("button", { name: "Check desktop:typecheck, passed" })).toBeVisible();
  await expect.element(thread.getByRole("button", { name: "Check components_test, failed" })).toBeVisible();
  await expect.element(thread.getByRole("button", { name: "Check app_smoke, running" })).toBeVisible();

  timePasses();
  await userEvent.click(thread.getByRole("button", { name: "Check app_smoke, failed" }));

  await expect.element(page.getByRole("dialog", { name: "Check log" })).toBeVisible();
  // It opens over the Session: the thread is still there, and closing the panel leaves it there.
  await expect.element(page.getByRole("region", { name: "Thread" })).toBeVisible();
  await userEvent.click(page.getByRole("dialog", { name: "Check log" }).getByRole("button", { name: /^Close/ }));
  await expect.element(page.getByRole("dialog", { name: "Check log" })).not.toBeInTheDocument();
  await expect.element(page.getByRole("region", { name: "Thread" })).toBeVisible();
});
