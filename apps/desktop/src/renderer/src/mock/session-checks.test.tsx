// A Session's Checks, through `App`: the thread draws a row per `armada check` the agent ran, and
// the one that was out ends. The walk `session-checks` draws it.

import { expect, test } from "vitest";
import { page, userEvent } from "vitest/browser";

import { mount, onScreen, toSession, unmountAfterEach } from "./testing";
import { timePasses } from "./time-passes";

unmountAfterEach();

test("Session checks: a running row ends", async () => {
  mount("session-checks");
  await onScreen();
  await toSession("Session check rows");

  const thread = page.getByRole("region", { name: "Thread" });
  await expect.element(thread.getByRole("button", { name: "Check desktop:typecheck, passed" })).toBeVisible();
  await expect.element(thread.getByRole("button", { name: "Check components_test, failed" })).toBeVisible();
  await expect.element(thread.getByRole("button", { name: "Check app_smoke, running" })).toBeVisible();

  timePasses();
  // The pointer is still over the Session's row in the list, whose hover card lies across the thread.
  await userEvent.unhover(page.getByRole("region", { name: "Jobs", exact: true }));
  await expect.poll(() => document.querySelector(".armada-inspect")).toBeNull();
  await expect.element(thread.getByRole("button", { name: "Check app_smoke, failed" })).toBeVisible();
});
