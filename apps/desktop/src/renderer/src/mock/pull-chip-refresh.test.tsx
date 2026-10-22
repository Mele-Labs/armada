// A right press on a pull request chip, in the Cockpit's merge line or in Progress, offers Refresh: Fleet reads the pull request
// from the forge now. Success shows only as the chip changing, by Fleet's events; a refusal is a toast. The walk `pull-chip-refresh`
// shows how it reads.

import { expect, test } from "vitest";
import { page, userEvent } from "vitest/browser";

import { CONFIG_PR, configFleet, configScenario } from "./sessions/pull-chip-refresh";
import { mount, onScreen, toSession, unmountAfterEach } from "./testing";

unmountAfterEach();

async function open() {
  const fleet = configFleet();
  mount(configScenario(fleet));
  await onScreen();
  return fleet;
}

const chip = () => page.getByRole("region", { name: "Merge queue" }).getByRole("button", { name: /^#2290 / });

function rightPress(target: Element): void {
  const box = target.getBoundingClientRect();
  target.dispatchEvent(new MouseEvent("contextmenu", { bubbles: true, cancelable: true, composed: true, view: window, clientX: box.x + 4, clientY: box.y + 4, button: 2 }));
}

const toast = (headline: string) => page.getByText(headline, { exact: true }).first();

test("a right press on the merge line chip offers Refresh under the pull request's number, and nothing else", async () => {
  await open();
  await expect.element(chip()).toBeVisible();
  rightPress(chip().element());
  await expect.element(page.getByRole("menu").getByText("Pull request #2290", { exact: true })).toBeVisible();
  await expect.element(page.getByRole("menuitem")).toHaveLength(1);
  await expect.element(page.getByRole("menuitem", { name: "Refresh" })).toBeVisible();
});

test("Refresh on the merge line chip asks Fleet once, by the repository's Manifest, and the chip reads failed from Fleet's events", async () => {
  const fleet = await open();
  await expect.element(chip()).toHaveAccessibleName(/ci running/);
  rightPress(chip().element());
  await userEvent.click(page.getByRole("menuitem", { name: "Refresh" }));
  expect(fleet.calls.refreshed).toEqual([{ manifestId: "armada", number: CONFIG_PR }]);
  await expect.element(chip()).toHaveAccessibleName(/ci failed/);
});

test("Shift+F10 on the focused chip opens the same menu", async () => {
  await open();
  chip().element().focus();
  await userEvent.keyboard("{Shift>}{F10}{/Shift}");
  await expect.element(page.getByRole("menuitem", { name: "Refresh" })).toBeVisible();
});

test("a refused Refresh is a toast in Fleet's words, and the chip stays as it was", async () => {
  const fleet = await open();
  fleet.refusesRefresh = { code: "fleet.pull_request_unreadable", message: "The forge did not answer." };
  rightPress(chip().element());
  await userEvent.click(page.getByRole("menuitem", { name: "Refresh" }));
  await expect.element(toast("The forge did not answer.")).toBeVisible();
  await expect.element(chip()).toHaveAccessibleName(/ci running/);
});

const progressChip = () => page.getByRole("region", { name: "Progress" }).getByRole("button", { name: /^Pull request #2290 / });

test("Refresh on the pull request in Progress asks Fleet once, and its Checks mark reads failed from Fleet's events", async () => {
  const fleet = await open();
  await toSession("Who sees your config");
  await expect.element(progressChip().getByRole("img", { name: /^Checks running/ })).toBeVisible();
  rightPress(progressChip().element());
  await expect.element(page.getByRole("menu").getByText("Pull request #2290", { exact: true })).toBeVisible();
  await userEvent.click(page.getByRole("menuitem", { name: "Refresh" }));
  expect(fleet.calls.refreshed).toEqual([{ manifestId: "armada", number: CONFIG_PR }]);
  await expect.element(progressChip().getByRole("img", { name: "Checks failed: ci / test" })).toBeVisible();
});

test("a refused Refresh in Progress is a toast too", async () => {
  const fleet = await open();
  await toSession("Who sees your config");
  fleet.refusesRefresh = { code: "fleet.pull_request_unreadable", message: "The forge did not answer." };
  rightPress(progressChip().element());
  await userEvent.click(page.getByRole("menuitem", { name: "Refresh" }));
  await expect.element(toast("The forge did not answer.")).toBeVisible();
});

const card = () => page.getByRole("tooltip");

test("the Job row on the merge line chip's card opens the Job that holds the pull request", async () => {
  await open();
  await userEvent.hover(chip());
  const job = card().getByRole("button", { name: "Open Job Who sees your config" });
  await expect.element(job).toBeVisible();
  await userEvent.click(job);
  await expect.element(page.getByRole("region", { name: /^Job: Who sees your config/ })).toBeVisible();
  await expect.element(card()).not.toBeInTheDocument();
});

test("a pull request no Job here holds has no Job row on its card", async () => {
  await open();
  await userEvent.hover(page.getByRole("region", { name: "Merge queue" }).getByRole("button", { name: /^#2291 / }));
  await expect.element(card().getByText("Bump the lockfile")).toBeVisible();
  await expect.element(card().getByText("Job", { exact: true })).not.toBeInTheDocument();
  await expect.element(card().getByRole("button")).not.toBeInTheDocument();
});

test("a failure read by Refresh is told to the Job that holds it once, however often it is read", async () => {
  await open();
  for (let time = 0; time < 2; time += 1) {
    rightPress(chip().element());
    await userEvent.click(page.getByRole("menuitem", { name: "Refresh" }));
    await expect.element(chip()).toHaveAccessibleName(/ci failed/);
  }
  await toSession("Who sees your config");
  const told = page.getByRole("region", { name: "Fleet turn" });
  await userEvent.click(told.getByRole("button"));
  await expect.element(told.getByRole("paragraph").first()).toHaveTextContent(/failed its checks at 4be1c07/);
  await expect.element(told).toHaveLength(1);
});
