// A Session's header counts the Checks its agent has out, and a press lists them. The walk
// `session-running-checks` draws it.

import { expect, test } from "vitest";
import { page, userEvent } from "vitest/browser";

import { mount, onScreen, toSession, unmountAfterEach } from "./testing";

unmountAfterEach();

async function openSession() {
  mount("session-running-checks");
  await onScreen();
  await toSession("Session running checks");
}

test("the header counts the running checks, and the one that passed is not among them", async () => {
  await openSession();
  await expect.element(page.getByRole("button", { name: "2 checks running" })).toBeVisible();
});

test("a press on the mark lists the running checks, and a press on one opens that run's log", async () => {
  await openSession();
  await userEvent.click(page.getByRole("button", { name: "2 checks running" }));
  const list = page.getByRole("menu", { name: "2 checks running" });
  await expect.element(list).toBeVisible();
  await expect.element(list.getByRole("menuitem", { name: "Check app_smoke, running" })).toBeVisible();

  await userEvent.click(list.getByRole("menuitem", { name: "Check test session_checking, running" }));
  const log = page.getByRole("dialog", { name: "Check log" });
  await expect.element(log).toBeVisible();
  // The panel is the run that was pressed (test, not app_smoke), and it is a running one: it can be stopped.
  await expect.element(log.getByText("test", { exact: true })).toBeVisible();
  expect(log.getByText("app_smoke").query()).toBeNull();
  await expect.element(log.getByRole("button", { name: "Stop", exact: true })).toBeVisible();
  expect(page.getByRole("menu").query()).toBeNull();
});
